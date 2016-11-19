use std::borrow::BorrowMut;
use std::io;
use std::io::{ErrorKind, Write};
use std::marker::PhantomData;

use bincode::SizeLimit;
use bincode::serde::{deserialize_from, serialize_into};

use futures::{Async, Fuse, Future, Join, Poll};

use serde::{Deserialize, Serialize, Serializer};

use thunderhead_store::{KvSource, KvSink, TdError};
use thunderhead_store::alloc::Scoped;
use thunderhead_store::util::{ByteReader, ByteWriter};

use datatype::datatypes::DatatypeId;

// TODO rename to Lens

/// A Lens is a bidirectional map from one type to another.
///
/// In normal usage, DataLenses are parameteric over their source/target stores. (Why?)
///
/// Lens has a generic argument S, because its Read/Write types may depend on S.
pub trait Lens<S>: Clone + Sized + 'static {
    // TODO: Target -> View
    // TODO: should target be the option? or should we have Future<Item = Option<Target>>?
    type Target;
    type ReadF: Future<Item = Self::Target, Error = TdError>;
    type WriteF: Future<Item = Self::Target, Error = TdError>;

    fn read<C: FutureChain<Option<Self::Target>, TdError>>(&self, source: &mut S, c: C) -> C::Out;

    fn write<V: Scoped<Self::Target>, C: FutureChain<(), TdError>>(&self, target: V, sink: &mut S, c: C) -> C::Out;
}

/// A datatype header.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DatatypeHeader {
    pub id: DatatypeId,
    pub version: u32,
}

/// Result of `SimpleLens::to_lens`.
pub struct SimpleLensToLens<S, L: SimpleLens> {
    inner: L,
    _phantom: PhantomData<S>,
}

impl<S, L: SimpleLens> Clone for SimpleLensToLens<S, L> {
    fn clone(&self) -> Self {
        SimpleLensToLens {
            inner: self.inner.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<S: KvSink + 'static, L: SimpleLens> Lens<S> for SimpleLensToLens<S, L> {
    type Target = L::Target;

    fn read<C: FutureChain<Option<Self::Target>, TdError>>(&self, source: &mut S, c: C) -> C::Out {
        let self_clone = self.clone();
        let reader_f = move |i: Option<S::Get>| i.map(|bs| self_clone.inner.read(bs.get().unwrap()));
        source.get([], premap_ok(move |i| reader_f(i), c))
    }

    fn write<V: Scoped<Self::Target>, C: FutureChain<(), TdError>>(&self, target: V, sink: &mut S, c: C) -> C::Out {
        // TODO: KvSink should support 'opening a writer', to avoid this heap allocation.
        let mut v = Vec::with_capacity(65536);
        unsafe { v.set_len(65536); }
        let newlen;
        {
            let mut buf = ByteWriter::wrap(v.borrow_mut());
            SimpleLens::write(&self.inner, target, &mut buf);
            newlen = buf.len_written();
        }
        unsafe { v.set_len(newlen); }

        sink.put_small([], v, c)
    }
}

/// A simple lens which wraps a Serde serializable.
pub trait SimpleLens: Clone + Sized + 'static {
    type Target: Serialize + Deserialize;

    /// TODO: we might consider making some kind of 'BorrowOrScoped' enum.
    fn read<B: AsRef<[u8]>>(&self, bytes: B) -> Self::Target;

    fn write<W: Write, V: Scoped<Self::Target>>(&self, t: V, sink: &mut W);

    /// Create a SimpleLens from this Lens.
    /// (SimpleDataLens doesn't impl DataLens for coherency reasons.)
    fn to_lens<S>(&self) -> SimpleLensToLens<S, Self> {
        SimpleLensToLens {
            inner: self.clone(),
            _phantom: PhantomData,
        }
    }
}

// MARK SerialLens

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

/// A data lens that reads/writes a Serde-serializable value, always prepended by the given header.
///
/// TODO: Generify this. ComboLens.
#[derive(Eq, PartialEq)]
pub struct SerialLens<T: Serialize + Deserialize + 'static> {
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

    fn read_header<B: AsRef<[u8]>>(&self, bytes: B) -> DatatypeHeader {
        // TODO: size check
        let len = bytes.as_ref().len() as u64;
        let mut reader = ByteReader::wrap(bytes.as_ref());
        // TODO: error handling
        // TODO: this is not particularly efficient
        let rw_value: SerialLensReadValue<T> = deserialize_from(&mut reader, SizeLimit::Bounded(len)).ok().unwrap();
        rw_value.header
    }
}

impl<T: Serialize + Deserialize + 'static> Clone for SerialLens<T> {
    fn clone(&self) -> SerialLens<T> {
        SerialLens {
            header: self.header.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<T: Serialize + Deserialize + 'static> SimpleLens for SerialLens<T> {
    type Target = T;

    fn read<B: AsRef<[u8]>>(&self, bytes: B) -> T {
        // TODO: size check
        let len = bytes.as_ref().len() as u64;
        let mut reader = ByteReader::wrap(bytes.as_ref());
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

/// A String lens, with no header.
pub struct StringLens<S: KvSink+ 'static> {
    _phantom: PhantomData<S>,
}

impl<S: KvSink+ 'static> StringLens<S> {
    fn new() -> Self {
        StringLens {
            _phantom: PhantomData,
        }
    }
}

impl<S: KvSink + 'static> Clone for StringLens<S> {
    fn clone(&self) -> Self {
        StringLens {
            _phantom: PhantomData,
        }
    }
}

impl<S: KvSink + 'static> SimpleLens for StringLens<S> {
    type Target = String;

    /// TODO: we might consider making some kind of 'BorrowOrScoped' enum.
    fn read<B: AsRef<[u8]>>(&self, source: B) -> Self::Target {
        String::from_utf8(Vec::from(source.as_ref())).ok().unwrap()
    }

    fn write<W: Write, V: Scoped<Self::Target>>(&self, t: V, sink: &mut W) {
       sink.write(t.get().unwrap().as_bytes()).ok();
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

// MARK LensWithHeader

// /// Helper for LensWithHeader,
// pub struct HeaderVerify<B: Scoped<[u8]>> {
//     header: DatatypeHeader,
//     _phantom: PhantomData<B>,
// }

// impl<B: Scoped<[u8]>> HeaderVerify<B> {
//     fn new(header: DatatypeHeader) -> Self {
//         HeaderVerify {
//             header: header,
//             _phantom: PhantomData,
//         }
//     }
// }

// impl<B: Scoped<[u8]>> FutureMap for HeaderVerify<B> {
//     type Input = Option<B>;
//     type Output = bool;
//     type Error = io::Error;

//     fn apply(&mut self, i: Self::Input) -> Result<Self::Output, Self::Error> {
//         match i {
//             Some(b) => {
//                 let inner_lens: SerialLens<()> = SerialLens::new(self.header);
//                 let h = inner_lens.read_header(b.get().unwrap());
//                 if h == self.header {
//                     Ok(true)
//                 } else {
//                     Err(io::Error::new(ErrorKind::InvalidData,
//                         format!("header mismatch: expected {:?} read {:?}", self.header, h)))
//                 }
//             },
//             None => Ok(false),
//         }
//     }
// }

// pub struct VerifyJoin<V, F, I> where
// V: Future<Item = bool, Error = io::Error>,
// F: Future<Item = Option<I>, Error = io::Error>,
// {
//     f1: Fuse<V>,
//     f2: F,
//     _phantom: PhantomData<I>,
// }

// impl<V, F, I> VerifyJoin<V, F, I> where
// V: Future<Item = bool, Error = io::Error>,
// F: Future<Item = Option<I>, Error = io::Error>,
// {
//     fn new(verify: V, future: F) -> Self {
//         VerifyJoin {
//             f1: verify.fuse(),
//             f2: future,
//             _phantom: PhantomData,
//         }
//     }
// }

// impl<V, F, I> Future for VerifyJoin<V, F, I> where
// V: Future<Item = bool, Error = io::Error>,
// F: Future<Item = Option<I>, Error = io::Error>,
// {
//     type Item = Option<I>;
//     type Error = io::Error;

//     fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
//         match self.f1.poll() {
//             Ok(Async::Ready(true)) => (),
//             Ok(Async::Ready(false)) => return Ok(Async::Ready(None)),
//             Ok(Async::NotReady) => return Ok(Async::NotReady),
//             Err(e) => return Err(e),
//         }

//         self.f2.poll()
//     }
// }

/// A Lens that wraps another lens while reading/writing a header at position [].
pub struct LensWithHeader<S: KvSink + 'static, L: Lens<S>> {
    header: DatatypeHeader,
    inner: L,
    _phantom: PhantomData<S>,
}

impl<S: KvSink, L: Lens<S>> Clone for LensWithHeader<S, L> {
    fn clone(&self) -> Self {
        LensWithHeader {
            header: self.header.clone(),
            inner: self.inner.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<S: KvSink + 'static, L: Lens<S>> Lens<S> for LensWithHeader<S, L> {
    type Target = L::Target;

    fn read<C: FutureChain<Option<Self::Target>, TdError>>(&self, source: &mut S, c: C) -> C::Out {
        let read_second = bind(|unit, c| self.inner.read(s, c), c);
        // Waiter::bind(
        //     |w| SerialLens::new(self.header).read(source, w),
        //     |header|
        // let second = self.inner.read(source, c)
        panic!("TODO")
    }

    fn write<V: Scoped<Self::Target>, C: FutureChain<(), TdError>>(&self, target: V, sink: &mut S, c: C) -> C::Out {
        panic!("TODO")
        // let header_lens: SerialLens<()> = SerialLens::new(self.header);
        // let first = exec(|c| header_lens.to_lens().write((), sink, c));
        // // TODO: subrange
        // // TODO: test subrange limitations
        // let second = self.inner.write(target, sink).to_future();
        // w.wait(first.join(second))
    }
}

#[cfg(test)]
mod tests {
    // use super::*;
    // use thunderhead_store::testlib::NullKeyDummyKvSink;

    // #[quickcheck]
    // fn test_string_lens(s: String) {
    //     let lens = StringLens::new();
    //     let repo = NullKeyDummyKvSink::new();
    // }
}
