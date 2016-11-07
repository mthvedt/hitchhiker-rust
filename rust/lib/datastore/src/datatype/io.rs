use std::borrow::BorrowMut;
use std::io;
use std::io::{ErrorKind, Write};
use std::marker::PhantomData;

use bincode::SizeLimit;
use bincode::serde::{deserialize_from, serialize_into};

use futures::{Future, Join};

use serde::{Deserialize, Serialize, Serializer};

use datatype::datatypes::DatatypeId;

use thunderhead_store::{KvSource, KvSink};
use thunderhead_store::alloc::Scoped;
use thunderhead_store::util::{ByteReader, ByteWriter};
use thunderhead_store::tdfuture::{FutureMap, FutureResult, FutureResultFuture, MapFuture};

// TODO rename to Lens

/// A Lens is a bidirectional map from one type to another.
///
/// In normal usage, DataLenses are parameteric over their source/target stores. (Why?)
pub trait Lens<S>: Clone + Sized {
    type Target;

    type ReadResult: Future<Item = Self::Target, Error = io::Error>;

    fn read(&self, source: &mut S) -> FutureResult<Self::ReadResult>;

    type WriteResult: Future<Item = (), Error = io::Error>;

    fn write<V: Scoped<Self::Target>>(&self, target: V, sink: &mut S) -> FutureResult<Self::WriteResult>;
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

    fn write<W: Write, V: Scoped<Self::Target>>(&self, t: V, sink: &mut W);

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
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct DatatypeHeader {
    pub id: DatatypeId,
    pub version: u32,
}

/// Helper for SerialLens read.
#[derive(Deserialize)]
struct SerialLensReadValue<T: Deserialize> {
    header: DatatypeHeader,
    value: T,
}

/// Helper for SerialLens write.
struct SerialLensWriteValue<'a, T: Serialize + 'a> {
    header: DatatypeHeader,
    value: &'a T,
}

impl<'a, T: Serialize + 'a> Serialize for SerialLensWriteValue<'a, T> {
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error> where S: Serializer {
        self.header.serialize(serializer).and_then(|_| self.value.serialize(serializer))
    }
}

/// A data lens that reads/writes a Serde-serializable value, with the given header, to the empty key.
#[derive(Eq, PartialEq)]
struct SerialLens<T: Serialize + Deserialize> {
    header: DatatypeHeader,
    _phantom: PhantomData<T>,
}

impl<T: Serialize + Deserialize> SerialLens<T> {
    pub fn new(header: DatatypeHeader) -> Self {
        SerialLens {
            header: header.clone(),
            _phantom: PhantomData,
        }
    }

    fn read_header<B: AsRef<[u8]>>(&self, source: B) -> DatatypeHeader {

        // TODO: size check
        let len = source.as_ref().len() as u64;
        let mut reader = ByteReader::wrap(source.as_ref());
        // TODO: error handling
        // TODO: this is not particularly efficient
        let rw_value: SerialLensReadValue<T> = deserialize_from(&mut reader, SizeLimit::Bounded(len)).ok().unwrap();
        rw_value.header
    }
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
        let rw_value: SerialLensReadValue<T> = deserialize_from(&mut reader, SizeLimit::Bounded(len)).ok().unwrap();
        assert!(rw_value.header == self.header);
        rw_value.value
    }

    fn write<W: Write, V: Scoped<Self::Target>>(&self, t: V, sink: &mut W) {
        let rw_value = SerialLensWriteValue {
            header: self.header.clone(),
            value: t.get().unwrap(),
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
struct SimpleLensRead<L: SimpleLens, B: Scoped<[u8]>> {
    lens: L,
    _p: PhantomData<B>,
}

impl<L: SimpleLens, B: Scoped<[u8]>> FutureMap for SimpleLensRead<L, B> {
    type Input = B;
    type Output = L::Target;
    type Error = io::Error;

    fn apply(&mut self, i: Self::Input) -> Result<Self::Output, io::Error> {
        Ok(self.lens.read(i.get().unwrap()))
    }
}

struct SimpleLensReified<S, L: SimpleLens> {
    inner: L,
    _phantom: PhantomData<S>,
}

impl<S, L: SimpleLens> Clone for SimpleLensReified<S, L> {
    fn clone(&self) -> Self {
        SimpleLensReified {
            inner: self.inner.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<S: KvSource + KvSink, L: SimpleLens> Lens<S> for SimpleLensReified<S, L> {
    type Target = L::Target;

    type ReadResult = MapFuture<S::Get, SimpleLensRead<L, S::GetValue>>;

    fn read(&self, source: &mut S) -> FutureResult<Self::ReadResult> {
        source.get([]).map(SimpleLensRead {
            lens: self.inner.clone(),
            _p: PhantomData,
        })
    }

    type WriteResult = S::PutSmall;

    fn write<V: Scoped<Self::Target>>(&self, target: V, sink: &mut S) -> FutureResult<Self::WriteResult> {
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

struct HeaderVerify<B: Scoped<[u8]>> {
    header: DatatypeHeader,
    _phantom: PhantomData<B>,
}

impl<B: Scoped<[u8]>> HeaderVerify<B> {
    fn new(header: DatatypeHeader) -> Self {
        HeaderVerify {
            header: header,
            _phantom: PhantomData,
        }
    }
}

impl<B: Scoped<[u8]>> FutureMap for HeaderVerify<B> {
    type Input = B;
    type Output = ();
    type Error = io::Error;

    fn apply(&mut self, i: Self::Input) -> Result<Self::Output, Self::Error> {
        let inner_lens: SerialLens<()> = SerialLens::new(self.header);
        let h = inner_lens.read_header(i.get().unwrap());
        if h == self.header {
            Ok(())
        } else {
            Err(io::Error::new(ErrorKind::InvalidData,
                format!("header mismatch: expected {:?} read {:?}", self.header, h)))
        }
    }
}

// TODO: macros for 'get' futures
struct GetNone<A, B> {
    _phantom: PhantomData<(A, B)>,
}

impl<A, B> GetNone<A, B> {
    fn new() -> Self {
        GetNone {
            _phantom: PhantomData,
        }
    }
}

impl<A, B> FutureMap for GetNone<A, B> {
    type Input = (A, B);
    type Output = ();
    type Error = io::Error;

    fn apply(&mut self, _unused: Self::Input) -> Result<Self::Output, Self::Error> {
        Ok(())
    }
}

struct GetSecond<A, B> {
    _phantom: PhantomData<(A, B)>,
}

impl<A, B> GetSecond<A, B> {
    fn new() -> Self {
        GetSecond {
            _phantom: PhantomData,
        }
    }
}

impl<A, B> FutureMap for GetSecond<A, B> {
    type Input = (A, B);
    type Output = B;
    type Error = io::Error;

    fn apply(&mut self, i: Self::Input) -> Result<Self::Output, Self::Error> {
        Ok(i.1)
    }
}

struct LensWithHeader<S, L: Lens<S>> {
    header: DatatypeHeader,
    inner: L,
    _phantom: PhantomData<S>,
}

impl<S: KvSource + KvSink, L: Lens<S>> Clone for LensWithHeader<S, L> {
    fn clone(&self) -> Self {
        LensWithHeader {
            header: self.header.clone(),
            inner: self.inner.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<S: KvSource + KvSink, L: Lens<S>> Lens<S> for LensWithHeader<S, L> {
    type Target = L::Target;

    // TODO: we probably want a 'FastJoin' or something that doesn't use inefficient space/copying like futures-rs Join.
    type ReadResult = MapFuture<
    Join<FutureResultFuture<MapFuture<S::Get, HeaderVerify<S::GetValue>>>, FutureResultFuture<L::ReadResult>>,
    GetSecond<(), L::Target>
    >;

    fn read(&self, source: &mut S) -> FutureResult<Self::ReadResult> {
        let first = source.get([]).map(HeaderVerify::new(self.header)).to_future();
        let second = self.inner.read(source).to_future();
        FutureResult::Wait(MapFuture::new(first.join(second), GetSecond::new()))
    }

    type WriteResult = MapFuture<
    Join<FutureResultFuture<S::PutSmall>, FutureResultFuture<L::WriteResult>>,
    GetNone<(), ()>>;

    fn write<V: Scoped<Self::Target>>(&self, target: V, sink: &mut S) -> FutureResult<Self::WriteResult> {
        let header_lens: SerialLens<()> = SerialLens::new(self.header);
        let first = header_lens.to_lens().write((), sink).to_future();
        let second = self.inner.write(target, sink).to_future();
        FutureResult::Wait(MapFuture::new(first.join(second), GetNone::new()))
    }
}
