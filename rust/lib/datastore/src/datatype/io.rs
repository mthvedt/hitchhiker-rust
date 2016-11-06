use std::borrow::{Borrow, BorrowMut};
use std::io::Write;
use std::marker::PhantomData;

use bincode::SizeLimit;
use bincode::serde::{deserialize_from, serialize_into};

use futures::{Async, Future, Join, Poll};

use serde::{Serialize, Deserialize};

use datatype::datatypes::DatatypeId;

use thunderhead_store::{KvSource, KvSink};
use thunderhead_store::alloc::Scoped;
use thunderhead_store::util::{ByteReader, ByteWriter};
use thunderhead_store::tdfuture::{FutureMap, FutureResult, MapFuture};

/// A Lens is a bidirectional map from one type to another.
///
/// In normal usage, DataLenses are parameteric over their source/target stores. (Why?)
trait Lens<S> {
    type Target;

    type ReadResult: Future<Item = Self::Target>;

    fn read(&self, source: &mut S) -> FutureResult<Self::ReadResult>;

    type WriteResult: Future<Item = ()>;

    fn write(&self, target: Self::Target, sink: &mut S) -> FutureResult<Self::WriteResult>;
}

struct SimpleLensReified<S, L: SimpleLens> {
    inner: L,
    _phantom: PhantomData<S>,
}

/// A simpler data lens, wrapping a Serde serializable.
///
/// TODO: This may be suboptimal for allocated types. Fortunately, the main path of Thunderhead
/// uses fixed-size types or types which require allocation anyway. Implementing a better story
/// for types which may use Scoped is a low-priority TODO.
trait SimpleLens: Clone + Sized {
    type Target: Serialize + Deserialize;

    /// TODO: we might consider making some kind of 'BorrowOrScoped' enum.
    fn read<B: AsRef<[u8]>>(&self, source: B) -> Self::Target;

    fn write<W: Write>(&self, t: Self::Target, sink: &mut W);

    /// Reify this into a DataLens.
    /// (SimpleDataLens doesn't impl DataLens for coherency reasons.)
    fn to_lens<S>(&self) -> SimpleLensReified<S, Self> {
        SimpleLensReified {
            inner: self.clone(),
            _phantom: PhantomData,
        }
    }
}

/// A datatype header.
#[derive(Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
struct DatatypeHeader {
    pub id: DatatypeId,
    pub version: u32,
}

/// Read-write value for SerialDataLens.
#[derive(Deserialize, Serialize)]
struct SerialLensRWValue<T: Serialize + Deserialize> {
    header: DatatypeHeader,
    value: T,
}

/// A data lens that reads/writes a value, with its header, to the key (null).
#[derive(Eq, PartialEq)]
struct SerialLens<T: Serialize + Deserialize> {
    header: DatatypeHeader,
    _phantom: PhantomData<T>,
}

impl<T: Serialize + Deserialize> Clone for SerialLens<T> {
    fn clone(&self) -> SerialLens<T> {
        SerialLens {
            header: self.header.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<T: Serialize + Deserialize> SimpleLens for SerialLens<T> {
    type Target = T;

    fn read<B: AsRef<[u8]>>(&self, source: B) -> T {
        // TODO: size check
        let len = source.as_ref().len() as u64;
        let mut reader = ByteReader::wrap(source.as_ref());
        // TODO: error handling
        let rw_value: SerialLensRWValue<T> = deserialize_from(&mut reader, SizeLimit::Bounded(len)).ok().unwrap();
        assert!(rw_value.header == self.header);
        rw_value.value
    }

    fn write<W: Write>(&self, t: Self::Target, sink: &mut W) {
        let rw_value = SerialLensRWValue {
            header: self.header.clone(),
            value: t,
        };

        // TODO get bounds from datastore
        // TODO error handling
        serialize_into(sink, &rw_value, SizeLimit::Bounded(65536)).ok().unwrap()
    }
}

// /// Helper to reify SerialDataLens::ReadResult into a named type.
// struct SingleKeyRead<F: Future<Item = B>, L: SimpleLens, B: Scoped<[u8]>> {
//     inner: F,
//     lens: L,
// }

// impl<F: Future<Item = B>, L: SimpleLens, B: Scoped<[u8]>> Future for SingleKeyRead<F, L, B> {
//     type Item = L::Target;
//     type Error = F::Error;

//     fn poll(&mut self) -> Poll<L::Target, F::Error> {
//         self.inner.poll().map(|async| async.map(|bytes| self.lens.read(bytes.get().unwrap())))
//     }
// }

/// Reification of SimpleDataLens::ReadResult into a named type.
struct SimpleDataLensRead<L: SimpleLens, B: Scoped<[u8]>> {
    lens: L,
    _p: PhantomData<B>,
}

impl<L: SimpleLens, B: Scoped<[u8]>> FutureMap for SimpleDataLensRead<L, B> {
    type Input = B;
    type Output = L::Target;

    fn apply(&mut self, i: Self::Input) -> Self::Output {
        self.lens.read(i.get().unwrap())
    }
}

impl<S: KvSource + KvSink, L: SimpleLens> Lens<S> for SimpleLensReified<S, L> {
    type Target = L::Target;

    type ReadResult = MapFuture<S::Get, SimpleDataLensRead<L, S::GetValue>>;

    fn read(&self, source: &mut S) -> FutureResult<Self::ReadResult> {
        let key = [];
        source.get(key).map(SimpleDataLensRead {
            lens: self.inner.clone(),
            _p: PhantomData,
        })
    }

    type WriteResult = S::PutSmall;

    fn write(&self, target: Self::Target, sink: &mut S) -> FutureResult<Self::WriteResult> {
        // Unavoidable heap allocation :(
        // TODO: KvSink should support 'opening a writer'.
        // TODO get capacity
        let mut v = Vec::with_capacity(65536);
        unsafe { v.set_len(65536); }
        let newlen;
        {
            let mut buf = ByteWriter::wrap(v.borrow_mut());
            SimpleLens::write(&self.inner, target, &mut buf);
            newlen = buf.len();
        }
        unsafe { v.set_len(newlen); }

        sink.put_small([], v)
    }
}

// /// We can't use futures::Map because we need to use this as an assacoiated type.
// struct SimpleLensFuture<B: Scoped<[u8]>, F: Future<Item = B>, S: SimpleLens> {
//     inner: F,
//     simple_lens: S,
// }

// impl<B: Scoped<[u8]>, F: Future<Item = B>, S: SimpleLens> Future for SimpleLensFuture<B, F, S> {
//     type Item = S::Target;
//     type Error = F::Error;

//     fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
//         match self.inner.poll() {
//             Ok(Async::Ready(bytes)) => Ok(Async::Ready(self.simple_lens.read(&bytes.get().unwrap()))),
//             Ok(Async::NotReady) => Ok(Async::NotReady),
//             Err(e) => Err(e),
//         }
//     }
// }

struct LensWithHeaderReified<S, L: LensWithHeader<S>> {
    inner: L,
    _phantom: PhantomData<S>,

}

// TODO: impl lens for LensWithHeaderReified

trait LensWithHeader<S>: Clone {
    type Sublens: Lens<S>;

    fn header(&self) -> DatatypeHeader;

    /// Reify this into a DataLens.
    /// (LensWithHeader doesn't impl DataLens for coherency reasons.)
    fn to_lens(&self) -> LensWithHeaderReified<S, Self> {
        LensWithHeaderReified {
            inner: self.clone(),
            _phantom: PhantomData,
        }
    }

    fn sublens(&self) -> Self::Sublens;
}
