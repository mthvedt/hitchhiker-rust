use std::borrow::{Borrow, BorrowMut};
use std::io::Write;
use std::marker::PhantomData;

use datatype::datatypes::DatatypeId;

use thunderhead_store::{KvRanged, KvSource, KvSink};
use thunderhead_store::alloc::Scoped;
use thunderhead_store::WrappingByteBuffer;
use thunderhead_store::tdfuture::{FutureMap, FutureResult, MapFuture};

use futures::{Async, Future, Join, Poll};

/// A DataLens is a bidirectional map from one type to another.
///
/// In normal usage, DataLenses are parameteric over their source/target stores. (Why?)
trait DataLens<S> {
    type Target;

    type ReadResult: Future<Item = Self::Target>;

    fn read(&self, source: &mut S) -> FutureResult<Self::ReadResult>;

    type WriteResult: Future<Item = ()>;

    fn write(&self, target: Self::Target, sink: &mut S) -> FutureResult<Self::WriteResult>;
}

struct SimpleDataLensReified<S, L: SimpleDataLens> {
    inner: L,
    _phantom: PhantomData<S>,
}

// TODO: impl

/// A simpler data lens.
trait SimpleDataLens: Clone + Sized {
    type Target;

    /// TODO: we might consider making some kind of 'BorrowOrScoped' enum.
    fn read<B: AsRef<[u8]>>(&self, source: B) -> Self::Target;

    fn len(&self, t: &Self::Target) -> usize;

    fn write<W: Write>(&self, sink: Self::Target, w: &mut W);

    /// Reify this into a DataLens.
    /// (SimpleDataLens doesn't impl DataLens for coherency reasons.)
    fn to_lens<S>(&self) -> SimpleDataLensReified<S, Self> {
        SimpleDataLensReified {
            inner: self.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Reification of SimpleDataLens::read into a named type.
struct SimpleDataLensRead<L: SimpleDataLens, B: Scoped<[u8]>> {
    lens: L,
    _p: PhantomData<B>,
}

impl<L: SimpleDataLens, B: Scoped<[u8]>> FutureMap for SimpleDataLensRead<L, B> {
    type Input = B;
    type Output = L::Target;

    fn apply(&mut self, i: Self::Input) -> Self::Output {
        self.lens.read(i.get().unwrap())
    }
}

impl<S: KvSource + KvSink, L: SimpleDataLens> DataLens<S> for L {
    type Target = L::Target;

    type ReadResult = MapFuture<S::Get, SimpleDataLensRead<L, S::GetValue>>;

    fn read(&self, source: &mut S) -> FutureResult<Self::ReadResult> {
        let key = [];
        source.get(key).map(SimpleDataLensRead {
            lens: self.clone(),
            _p: PhantomData,
        })
    }

    type WriteResult = S::PutSmall;

    fn write(&self, target: Self::Target, sink: &mut S) -> FutureResult<Self::WriteResult> {
        // Unavoidable heap allocation :(
        // TODO: KvSink should support 'opening a writer'.
        let len = self.len(&target);
        let mut v = Vec::with_capacity(len);
        unsafe { v.set_len(len); }
        let mut buf = WrappingByteBuffer::wrap(v.borrow_mut());
        SimpleDataLens::write(self, target, &mut buf);

        sink.put_small([], v)
    }
}

// TODO should serializers be objects?

// TODO: struct?

/// We can't use futures::Map because we need to use this as an assacoiated type.
struct SimpleDataLensFuture<B: Scoped<[u8]>, F: Future<Item = B>, S: SimpleDataLens> {
    inner: F,
    simple_lens: S,
}

impl<B: Scoped<[u8]>, F: Future<Item = B>, S: SimpleDataLens> Future for SimpleDataLensFuture<B, F, S> {
    type Item = S::Target;
    type Error = F::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.inner.poll() {
            Ok(Async::Ready(bytes)) => Ok(Async::Ready(self.simple_lens.read(&bytes.get().unwrap()))),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err(e),
        }
    }
}

#[derive(Clone)]
struct DatatypeHeaderLens {
    inner: DatatypeHeader,
}

impl SimpleDataLens for DatatypeHeaderLens {
    type Target = DatatypeHeader;

    /// TODO: we might consider making some kind of 'BorrowOrScoped' enum.
    fn read<B: AsRef<[u8]>>(&self, input: B) -> Self::Target {

    }

    fn len(&self, _ignored: &Self::Target) -> usize {
        8
    }

    fn write<W: Write>(&self, target: Self::Target, w: &mut W) {

    }
}

struct LensWithHeaderReified<S, L: LensWithHeader<S>> {
    inner: L,
    _phantom: PhantomData<S>,

}

/// A datatype header.
#[derive(Clone, Copy, Eq, PartialEq)]
struct DatatypeHeader {
    pub id: DatatypeId,
    pub version: u32,
}

// TODO: impl

trait LensWithHeader<S>: Clone {
    type Sublens: DataLens<S>;

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
