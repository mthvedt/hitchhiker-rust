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

// TODO rename to Lens

/// A Lens is a bidirectional map from one type to another.
///
/// In normal usage, DataLenses are parametric over their source/target stores. (Why?)
///
/// N.B. What we really want is a LensRead and LensWrite pair of traits, where Lens
/// implements Read and Write. Hmm... is it possible?
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
pub trait SimpleLens: Clone + Sized + 'static {
    type Target;

    /// TODO: we might consider making some kind of 'BorrowOrScoped' enum.
    fn read<Bytes: AsRef<[u8]>>(&self, source: Bytes) -> Self::Target;

    fn write<W: Write, V: Scoped<Self::Target>>(&self, t: V, sink: &mut W);

    /// Reify this into a Lens.
    /// (SimpleLens doesn't impl Lens for coherency reasons.)
    fn to_lens<S>(&self) -> SimpleLensWrapper<S, Self> {
        SimpleLensWrapper {
            inner: self.clone(),
            _phantom: PhantomData,
        }
    }
}

/// Implementation detail for SimpleLens::read.
pub struct SimpleLensRead<L: SimpleLens, B: Scoped<[u8]>> {
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

/// Implementation detail for SimpleLens::to_lens.
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

impl<S: KvSource + KvSink, L: SimpleLens> Lens<S> for SimpleLensWrapper<S, L> {
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

impl<S: KvSource + KvSink, T: 'static + ?Sized + Serialize + Deserialize> Lens<S> for BincodeLens<T> {
    type Target = T;
    type ReadResult = <SimpleLensWrapper<S, BincodeLensInner<T>> as Lens<S>>::ReadResult;
    type WriteResult = <SimpleLensWrapper<S, BincodeLensInner<T>> as Lens<S>>::WriteResult;

    fn read(&self, source: &mut S) -> Self::ReadResult {
        self.inner.to_lens().read(source)
    }

    fn write<V: Scoped<Self::Target>>(&self, target: V, sink: &mut S) -> Self::WriteResult {
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
    fn read<Bytes: AsRef<[u8]>>(&self, source: Bytes) -> Self::Target {
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

impl<S: KvSource + KvSink> Lens<S> for StringLens {
    type Target = String;
    type ReadResult = <SimpleLensWrapper<S, StringLensInner> as Lens<S>>::ReadResult;
    type WriteResult = <SimpleLensWrapper<S, StringLensInner> as Lens<S>>::WriteResult;

    /// TODO: consume the bytes directly in StringLens, instead of copying.
    fn read(&self, source: &mut S) -> Self::ReadResult {
        StringLensInner.to_lens().read(source)
    }

    fn write<V: Scoped<Self::Target>>(&self, target: V, sink: &mut S) -> Self::WriteResult {
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
    fn read<Bytes: AsRef<[u8]>>(&self, source: Bytes) -> Self::Target {
        String::from_utf8_lossy(source.as_ref()).into_owned()
    }

    fn write<W: Write, V: Scoped<Self::Target>>(&self, t: V, sink: &mut W) {
        sink.write_all(t.get().unwrap().as_ref()).ok().unwrap();
    }
}

// No tests here. Tests are on implementations of Lens.
