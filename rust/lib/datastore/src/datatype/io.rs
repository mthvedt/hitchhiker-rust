use std::borrow::BorrowMut;
use std::io;
use std::io::{ErrorKind, Write};
use std::marker::PhantomData;

use bincode::SizeLimit;
use bincode::serde::{deserialize_from, serialize_into};

use futures::{Future, Join};

use serde::{Deserialize, Serialize, Serializer};

use datatype::datatypes::DatatypeId;

use thunderhead_store::{KvSource, KvSink, TdError};
use thunderhead_store::alloc::Scoped;
use thunderhead_store::util::{ByteReader, ByteWriter};
use thunderhead_store::tdfuture::{FutureMap, MapFuture};

// TODO rename to Lens

/// A Lens is a bidirectional map from one type to another.
///
/// In normal usage, DataLenses are parametric over their source/target stores. (Why?)
pub trait Lens<S>: Clone + Sized {
    type Target: 'static;

    type ReadResult: Future<Item = Option<Self::Target>, Error = TdError> + 'static;

    fn read(&self, source: &mut S) -> Self::ReadResult;

    type WriteResult: Future<Item = (), Error = TdError> + 'static;

    fn write<V: Scoped<Self::Target>>(&self, target: V, sink: &mut S) -> Self::WriteResult;
}

/// A simpler data lens, wrapping a Serde serializable.
///
/// TODO: This may be suboptimal for allocated types. Fortunately, the main path of Thunderhead
/// uses fixed-size types or types which require allocation anyway. Implementing a better story
/// for types which may use Scoped is a low-priority todo.
trait SimpleLens: Clone + Sized + 'static {
    type Target: Serialize + Deserialize;

    /// TODO: we might consider making some kind of 'BorrowOrScoped' enum.
    fn read<Bytes: AsRef<[u8]>>(&self, source: Bytes) -> Self::Target;

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
struct SerialLens<T: Serialize + Deserialize + 'static> {
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

    fn read_header<Bytes: AsRef<[u8]>>(&self, source: Bytes) -> DatatypeHeader {
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

    fn read<Bytes: AsRef<[u8]>>(&self, source: Bytes) -> T {
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

/// Reification of SimpleDataLens::ReadResult into a named type.
struct SimpleLensRead<L: SimpleLens, B: Scoped<[u8]>> {
    lens: L,
    _p: PhantomData<B>,
}

impl<L: SimpleLens, B: Scoped<[u8]>> FutureMap for SimpleLensRead<L, B> {
    type Input = Option<B>;
    type Output = Option<L::Target>;
    type Error = TdError;

    fn apply(&mut self, iopt: Self::Input) -> Result<Self::Output, TdError> {
        match iopt {
            Some(i) => Ok(Some(self.lens.read(i.get().unwrap()))),
            None => Ok(None),
        }
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

    type ReadResult = MapFuture<S::GetF, SimpleLensRead<L, S::Get>>;

    fn read(&self, source: &mut S) -> Self::ReadResult {
        MapFuture::new(source.get([]), SimpleLensRead {
            lens: self.inner.clone(),
            _p: PhantomData,
        })
    }

    type WriteResult = S::PutF;

    fn write<V: Scoped<Self::Target>>(&self, target: V, sink: &mut S) -> Self::WriteResult {
        // Unavoidable heap allocation :(
        // TODO: KvSink should support 'opening a writer'.
        // TODO get capacity
        // TODO simplify this
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
    type Input = Option<B>;
    type Output = Option<()>;
    type Error = TdError;

    fn apply(&mut self, iopt: Self::Input) -> Result<Self::Output, Self::Error> {
        let i = match iopt {
            Some(i) => i,
            None => return Ok(None),
        };

        let inner_lens: SerialLens<()> = SerialLens::new(self.header);
        let h = inner_lens.read_header(i.get().unwrap());
        if h == self.header {
            Ok(Some(()))
        } else {
            // TODO: better errors everywhere
            Err(TdError::from(io::Error::new(ErrorKind::InvalidData,
                format!("header mismatch: expected {:?} read {:?}", self.header, h))))
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
    type Error = TdError;

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
    type Input = (Option<A>, Option<B>);
    type Output = Option<B>;
    type Error = TdError;

    fn apply(&mut self, i: Self::Input) -> Result<Self::Output, Self::Error> {
        match i {
            (Some(_), Some(b)) => Ok(Some(b)),
            (None, None) => Ok(None),
            (Some(_), None) => Err(TdError::from(io::Error::new(ErrorKind::InvalidData,
                format!("found header but no data")))),
            (None, Some(_)) => Err(TdError::from(io::Error::new(ErrorKind::InvalidData,
                format!("expected header, found data but no header")))),
        }
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

    type ReadResult = MapFuture<
    Join<MapFuture<S::GetF, HeaderVerify<S::Get>>, L::ReadResult>, GetSecond<(), L::Target>
    >;

    fn read(&self, source: &mut S) -> Self::ReadResult {
        let first = MapFuture::new(source.get([]), HeaderVerify::new(self.header));
        let second = self.inner.read(source);
        MapFuture::new(first.join(second), GetSecond::new())
    }

    type WriteResult = MapFuture<Join<S::PutF, L::WriteResult>, GetNone<(), ()>>;

    fn write<V: Scoped<Self::Target>>(&self, target: V, sink: &mut S) -> Self::WriteResult {
        let header_lens: SerialLens<()> = SerialLens::new(self.header);
        let first = header_lens.to_lens().write((), sink);
        let second = self.inner.write(target, sink);
        MapFuture::new(first.join(second), GetNone::new())
    }
}

// No tests here. Tests are on implementations of Lens.
