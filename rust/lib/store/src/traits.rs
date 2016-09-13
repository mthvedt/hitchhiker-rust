use std::io;

use byteorder::{ByteOrder, LittleEndian};
use futures;

pub enum ErrorType {
    Other,
}

pub struct Error {
    t: ErrorType,
    // TODO: can we make this StringLike for perf? to what extent does
    // perf matter?
    message: String,
}

impl Error {
    pub fn other(s: &str) -> Error {
        Error { t: ErrorType::Other, message: String::from(s) }
    }
}

// TODO: might not need static
pub trait Future<T: Send + 'static> : futures::Future<Item = T, Error = Error> {}
// TODO why doesn't this work?
//impl<T> Future<T> for futures::Future<Item = T, Error = Error> 
//where T: Send + 'static {}
impl<F, T> Future<T> for F where
T: Send + 'static,
F: futures::Future<Item = T, Error = Error> {}

pub trait DataWrite {
    type R: Future<()>;
    fn write(&mut self, buf: &[u8]) -> Self::R;
}

pub trait Datum: Send {
    /*
      A datum has a max size of 64kbytes. Thunderhead is not designed
      for very large values; they should be split into multiple values
      instead.
      */
    fn len(&self) -> u16;
    // TODO: this is inelegant. Instead Datum should require Sized
    fn write_bytes<W: DataWrite>(&self, w: &mut W) -> W::R where Self:Sized;

    // TODO: a way to get the raw slice in a way that's possiby zero-copy
}

pub trait KvSink {
    type R: Future<()>;
    fn write(&self, k: &Datum, v: &Datum) -> Self::R;
}

pub trait KvDoc {
    fn write<S: KvSink>(&self, r: &S) -> S::R;
}

pub trait KvSource {
    type D: Datum + 'static;
    type R: Future<Self::D>;
    fn read(&self, k: &Datum) -> Self::R;
}

pub trait KvCursor {
    type D: Datum + 'static;
    type R: Future<Self::D>;
    fn get(&self) -> Self::R;
    fn next(self) -> KvCursor<D = Self::D, R = Self::R>;
}

pub trait KvStream {
    type D: Datum + 'static;
    type R: Future<Self::D>;
    fn cursor(&self) -> KvCursor<D = Self::D, R = Self::R>;
}

// TODO: consider perf consequences of making this variable-sized
// TODO: if the above, make this u128
#[derive(PartialEq, Eq, Hash, Clone)]
pub struct Counter {
    // little-endian
    data: u64,
}

impl Counter {
    pub fn new(x: u64) -> Counter {
        Counter { data: x }
    }

    pub fn inc(&self) -> Counter {
        Counter { data: self.data + 1 }
    }
}

impl Datum for Counter {
    fn len(&self) -> u16 {
        8
    }

    fn write_bytes<W: DataWrite>(&self, w: &mut W) -> W::R {
        let mut tmp = [0 as u8; 8];
        LittleEndian::write_u64(&mut tmp, self.data);
        w.write(&tmp)
    }
}

// TODO: ???
// We want ephemeral snapshots to be addressable by pointer, for speed.
// We also need coordinated ephemeral snapshots... addressable by single indirect...
//
// In general, the futures lib cannot be relied upon to be non-static.
// Everything that comes out of here *must* be compatible with static lifetimes.
// This probably requires unsafety? Core handles/pointers can't have static references,
// so we need to use pointers instead.
// (There's also the question of how to invalidate the pointers upon closing the undelrying
// datastore...)
/// SnapshotStore is not lifetimed. In an ideal world, it would be lifetimed
/// such that no use-after-free operations are permitted. There are three issues with this:
/// 1) The futures-rs library, which we use heavily, does not play nicely with finite lifetimes.
/// 2) Sub-lifetimes on associated types are difficult in Rust unitl higher-kinded lifetimes
/// are implemented (if they ever will be). Workarounds exist in the form of 'reference traits'
/// like IntoIterator, but these are clumsy in our use case.
/// 3) Errors may cause production SnapshotStores to abruptly close so we have to handle
/// that case anyway.
pub trait SnapshotStore {
    // TODO: persistent snapshot cursors.
    // TODO: what is the correct use of &mut self?

    // These are not reference-counted. Use cases interested in
    // tracking reference counts (or other snapshot metadata)
    // must do so by separate means.

    // TODO: instead, use rust safety to eliminate use-after-free
    // for snapshots...?
    type Snap: KvSource;
    fn open(&mut self) -> Counter;
    fn close(&mut self, stamp: &Counter);
    //fn diff(&self, &prev: SnapshotStamp) -> KvStream;
    // TODO make this safer? what is the semantics of a removed snapshot?
    fn snap(&self, stamp: &Counter) -> Option<Self::Snap>;
}

