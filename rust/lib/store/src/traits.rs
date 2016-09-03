// TODO rename this file

trait Datum {
    fn to_slice(&self) -> &[i8];
    fn to_slice_mut(&mut self) -> &mut [i8];
}

enum KvResult<T> {
    Success(T),
    // TODO: can we make this StringLike for perf?
    Failure(String)
}

// TODO all these should be generic
trait KvDoc {
    fn write(&self, r: &KvSink) -> KvResult<()>;
}

trait KvSink {
    fn write(&self, k: &Datum, v: &Datum) -> KvResult<()>;
}

trait KvSource {
    fn read(&self, k: &Datum) -> KvResult<&Datum>;
}

trait KvCursor {
    fn get(&self) -> KvResult<&Datum>;
    fn next(self) -> KvCursor;
}

trait KvStream {
    fn cursor(&self) -> KvCursor;
}

trait SnapshotStamp {
    // TODO extend datum. This is just an impl of Datum
    // that's type-safed. Might be better as a struct.
}

trait SnapshotStore {
    // TODO: persistent snapshot cursors.
    // TODO: what is the correct use of &mut self?

    // These are not reference-counted. Use cases interested in
    // tracking reference counts (or other snapshot metadata)
    // must do so by separate means.

    // TODO: instead, use rust safety to eliminate use-after-free
    // for snapshots...?
    type Source : KvSource + Sized;
    fn open(&mut self) -> SnapshotStamp;
    fn close(&mut self, stamp: &SnapshotStamp);
    //fn diff(&self, &prev: SnapshotStamp) -> KvStream;
    fn snap(&self, stamp: &SnapshotStamp) -> Option<Self::Source>;
}

// TODO: Ephemeral kv sink. Good perf for event model POC.
// TODO: permanent kv sink, for obvious reasons. Can we mmap the ephem
// into the permanent?

