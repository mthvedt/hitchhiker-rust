use bincode::SizeLimit;
use bincode::serde::{deserialize_from, serialize_into};

// TODO: maybe a thunderhead_util lib?
use thunderhead_store::util::{ByteReader, ByteWriter};

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
// TODO: we can generalize this. We can write a serde lens that doesn't rely on header,
// then wrap it.
pub struct BincodeLens<T: Serialize + Deserialize + 'static> {
    header: DatatypeHeader,
    _phantom: PhantomData<T>,
}

impl<T: Serialize + Deserialize> BincodeLens<T> {
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

impl<T: Serialize + Deserialize> Clone for BincodeLens<T> {
    fn clone(&self) -> SerialLens<T> {
        SerialLens {
            header: self.header.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<T: Serialize + Deserialize> SimpleLens for BincodeLens<T> {
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
