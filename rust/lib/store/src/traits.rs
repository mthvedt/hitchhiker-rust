// TODO might not need byteorder.
use futures;

use data::*;

use tree::Counter;

pub enum ErrorType {
    NotFound,
    Other,
}

#[allow(dead_code)]
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

pub type Done<T> = futures::Done<T, Error>;

// TODO: should this be exposed?
pub fn ok<T>(t: T) -> Done<T> {
    futures::done(Ok(t))
}

pub fn err<T>(e: Error) -> Done<T> {
    futures::done(Err(e))
}

/*
pub trait KvDoc {
    fn write<S: KvSink>(&self, r: &S) -> S::R;
}
*/

pub trait KvSource {
    // TODO: should this require static? what if the future doesn't?
    type D: Datum + Send + 'static;
    type R: Future<Self::D>;
    fn read<DR: Datum>(&self, k: &DR) -> Self::R;
}

pub trait KvSink {
    type R: Future<()>;
    fn write<D1: Datum, D2: Datum>(&mut self, k: &D1, v: &D2) -> Self::R;
}

/*
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
*/

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
    //
    // TODO: cursors are a core feature also
    type Snap: KvSource + Send + 'static;
    // TODO different types for each
    type SnapTmp: KvSource + Send + 'static;
    // TODO different types for each
    type SnapMut: KvSource + KvSink + Send + 'static;

    /// Recover a permanent snapshot given that snapshot's counter.
    type SnapF: Future<Self::Snap>;
    fn snap(&self, stamp: &Counter) -> Self::SnapF;

    /// Open a permanent snapshot for read-only transactions.
    /// The snapshot is guaranteed to be durable until closed.
    type SnapNewF: Future<Self::Snap>;
    fn snap_new(&mut self) -> Self::SnapNewF;

    type SnapTmpF: Future<Self::SnapTmp>;
    fn snap_tmp(&mut self) -> Self::SnapTmp;

    // TODO: we need cursors also.

    // TODO: this isn't necessary for POC, but it (or something like it) should be implemented
    // eventually. Remember to think about the distributed case when implementing.
    // Open an ephemeral snapshot. These snapshots don't bookkeep reads and writes,
    // and vanish when the database closes.
    //type SnapEphemF: Future<Self::SnapEphem>;
    //fn snap_ephem(&self) -> Self:SnapEphemF;

    /// Open an ephemeral, mutable snapshot, used for read-write transactions.
    ///
    /// Implementations should probably have some safety check, where calling Drop
    /// on an unclosed or undiscarded snapshot is an error.
    type SnapMutF: Future<Self::SnapMut>;
    fn snap_mut(&mut self) -> Self::SnapMutF;

    type SnapCloseF: Future<()>;
    fn snap_close(&mut self, stamp: &Counter) -> Self::SnapCloseF;

    type CloseF: Future<()>;
    fn close(&mut self) -> Self::CloseF;
}
