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

    fn read(&self, source: S) -> Self::ReadResult {
        let first = MapFuture::new(source.get([]), HeaderVerify::new(self.header));
        let second = self.inner.read(source);
        MapFuture::new(first.join(second), GetSecond::new())
    }

    type WriteResult = MapFuture<Join<S::PutF, L::WriteResult>, GetNone<(), ()>>;

    fn write<V: Scoped<Self::Target>>(&self, target: V, sink: S) -> Self::WriteResult {
        let header_lens: SerialLens<()> = SerialLens::new(self.header);
        let first = header_lens.to_lens().write((), sink);
        let second = self.inner.write(target, sink);
        MapFuture::new(first.join(second), GetNone::new())
    }
}
