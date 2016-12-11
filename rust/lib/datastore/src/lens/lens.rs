// TODO: might be more appropriate to have lens as part of store library

use std::borrow::BorrowMut;
use std::io::Write;
use std::marker::PhantomData;

use bincode::SizeLimit;
use bincode::serde::{deserialize_from, serialize_into};

use futures::Future;

use serde::{Deserialize, Serialize};

use thunderhead_store::{KvSource, KvSink, TdError};
use thunderhead_store::alloc::Scoped;
// TODO: maybe a thunderhead_util lib?
use thunderhead_store::util::{ByteReader, ByteWriter};
use thunderhead_store::tdfuture::{FutureMap, MapFuture};

// TODO: this is wrong. The whole concept of Lens is wrong because they're not bidirectional. Fuck fuck fuck

/*
TODO

Use stream transformers that can be composed, not lenses.
Something like nom, but the input is always a stream.

trait Transformer<Io> {
  type Target: 'static;

  fn pipe<W: Write>(&self, io: &Io, w: &W) -> Result of some kind

  fn read(&self, io: &Io) -> Result<Option<Scoped result>, TdError>

  fn write<W: Write>(&self, t: &Scoped<Target>, w: &W) -> Result of some kind
}
*/

// TODO: all this lens crap is overengineered. This library (and map) both need massive simplification.

/// TODO: add lenses for streaming sources. We want BOTH static lenses and streaming lenses (why?)

// TODO: how about ReadLens -> Read, WriteLens -> Write? Because these are not actually lenses.
pub trait ReadLens<S>: Clone + Sized {
    type Target: 'static;

    type ReadResult: Future<Item = Option<Self::Target>, Error = TdError> + 'static;

    fn read(&self, source: S) -> Self::ReadResult;
}

pub trait WriteLens<S>: Clone + Sized {
    type Target: 'static;

    type WriteResult: Future<Item = (), Error = TdError> + 'static;

    fn write<V: Scoped<Self::Target>>(&self, target: V, sink: S) -> Self::WriteResult;
}

/// A simple implementation of `ReadLens` and `WriteLens`, for a type that can be converted
/// to/from bytes directly.

// TODO: this blocks, and is therefore bad.
pub trait SimpleLens: Clone + Sized + 'static {
    type Target;

    fn read<Bytes: AsRef<[u8]>>(&self, source: &Bytes) -> Self::Target;

    fn write<W: Write, V: Scoped<Self::Target>>(&self, t: V, sink: &mut W);

    /// Reify this into a Read+WriteLens.
    /// (SimpleLens doesn't impl these for coherency reasons.)
    fn to_lens<S>(&self) -> SimpleLensWrapper<S, Self> {
        SimpleLensWrapper {
            inner: self.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Implementation detail for SimpleLens::read.
pub struct SimpleLensReadMap<L: SimpleLens, B: Scoped<[u8]>> {
    lens: L,
    _p: PhantomData<B>,
}

impl<L: SimpleLens, B: Scoped<[u8]>> FutureMap for SimpleLensReadMap<L, B> {
    type Input = Option<B>;
    type Output = Option<L::Target>;
    type Error = TdError;

    fn apply(&mut self, iopt: Self::Input) -> Result<Self::Output, TdError> {
        match iopt {
            Some(i) => Ok(Some(self.lens.read(&i.get().unwrap()))),
            None => Ok(None),
        }
    }
}

/// Implementation detail for `SimpleLens::to_lens`.
pub struct SimpleLensWrapper<S, L: SimpleLens> {
    inner: L,
    _phantom: PhantomData<S>,
}

impl<S, L: SimpleLens> Clone for SimpleLensWrapper<S, L> {
    fn clone(&self) -> Self {
        SimpleLensWrapper {
            inner: self.inner.clone(),
            _phantom: PhantomData,
        }
    }
}

impl<S: KvSource, L: SimpleLens> ReadLens<S> for SimpleLensWrapper<S, L> {
    type Target = L::Target;

    type ReadResult = MapFuture<S::GetF, SimpleLensReadMap<L, S::Get>>;

    fn read(&self, mut source: S) -> Self::ReadResult {
        MapFuture::new((&mut source).get([]), SimpleLensReadMap {
            lens: self.inner.clone(),
            _p: PhantomData,
        })
    }
}

impl<S: KvSink, L: SimpleLens> WriteLens<S> for SimpleLensWrapper<S, L> {
    type Target = L::Target;

    type WriteResult = S::PutF;

    fn write<V: Scoped<Self::Target>>(&self, target: V, mut sink: S) -> Self::WriteResult {
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

/// A lens that uses serde serialize/deserialize to write to/from bincode.
pub struct BincodeLens<T: ?Sized> {
    inner: BincodeLensInner<T>,
}

impl<T: ?Sized> BincodeLens<T> {
    pub fn new() -> Self {
        BincodeLens {
            inner: BincodeLensInner::new(),
        }
    }
}

impl<T: ?Sized> Clone for BincodeLens<T> {
    fn clone(&self) -> Self {
        BincodeLens::new()
    }
}

impl<S: KvSource, T: 'static + ?Sized + Serialize + Deserialize> ReadLens<S> for BincodeLens<T> {
    type Target = T;
    type ReadResult = <SimpleLensWrapper<S, BincodeLensInner<T>> as ReadLens<S>>::ReadResult;

    fn read(&self, source: S) -> Self::ReadResult {
        self.inner.to_lens().read(source)
    }
}

impl<S: KvSink, T: 'static + ?Sized + Serialize + Deserialize> WriteLens<S> for BincodeLens<T> {
    type Target = T;
    type WriteResult = <SimpleLensWrapper<S, BincodeLensInner<T>> as WriteLens<S>>::WriteResult;

    fn write<V: Scoped<Self::Target>>(&self, target: V, sink: S) -> Self::WriteResult {
        self.inner.to_lens().write(target, sink)
    }
}

/// Implementation detail for BincodeLens.
pub struct BincodeLensInner<T: ?Sized> {
    _p: PhantomData<*const T>,
}

impl<T: ?Sized> BincodeLensInner<T> {
    fn new() -> Self {
        BincodeLensInner {
            _p: PhantomData,
        }
    }
}

impl<T: ?Sized> Clone for BincodeLensInner<T> {
    fn clone(&self) -> Self {
        BincodeLensInner::new()
    }
}

impl<T: 'static + ?Sized + Serialize + Deserialize> SimpleLens for BincodeLensInner<T> {
    type Target = T;

    /// TODO: we might consider making some kind of 'BorrowOrScoped' enum.
    fn read<Bytes: AsRef<[u8]>>(&self, source: &Bytes) -> Self::Target {
        // TODO: size check
        let len = source.as_ref().len() as u64;
        let mut reader = ByteReader::wrap(source.as_ref());
        // TODO: error handling
        deserialize_from(&mut reader, SizeLimit::Bounded(len)).ok().unwrap()
    }

    // TODO: write should return success/failure
    fn write<W: Write, V: Scoped<Self::Target>>(&self, t: V, sink: &mut W) {
        // TODO get bounds from datastore
        // TODO error handling
        serialize_into(sink, t.get().unwrap(), SizeLimit::Bounded(65536)).ok().unwrap()
    }
}

/// A lens that reads/writes strings as raw bytes.
#[derive(Clone)]
pub struct StringLens;

// TODO: this copies, which is inefficient.
impl<S: KvSource> ReadLens<S> for StringLens {
    type Target = String;
    type ReadResult = <SimpleLensWrapper<S, StringLensInner> as ReadLens<S>>::ReadResult;

    /// TODO: consume the bytes directly in StringLens, instead of copying.
    fn read(&self, source: S) -> Self::ReadResult {
        StringLensInner.to_lens().read(source)
    }
}

impl<S: KvSink> WriteLens<S> for StringLens {
    type Target = String;
    type WriteResult = <SimpleLensWrapper<S, StringLensInner> as WriteLens<S>>::WriteResult;

    fn write<V: Scoped<Self::Target>>(&self, target: V, sink: S) -> Self::WriteResult {
        StringLensInner.to_lens().write(target, sink)
    }
}


/// Implementation detail for StringLens.
#[derive(Clone)]
pub struct StringLensInner;

impl SimpleLens for StringLensInner {
    type Target = String;

    /// TODO: consume the bytes directly in StringLens, instead of copying.
    /// TODO: support errors.
    fn read<Bytes: AsRef<[u8]>>(&self, source: &Bytes) -> Self::Target {
        String::from_utf8_lossy(source.as_ref()).into_owned()
    }

    fn write<W: Write, V: Scoped<Self::Target>>(&self, t: V, sink: &mut W) {
        sink.write_all(t.get().unwrap().as_ref()).ok().unwrap();
    }
}

// // TODO: this is why we want LensRead
// pub trait ScopedLensRead<A, B> {
//     type Read: Scoped<A>;
//
//     fn read<SB: Scoped<B>>(&self, sb: SB) -> Self::Read;
// }

// No tests here. Tests are on implementations of Lens.
