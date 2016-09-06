// TODO rename this file

use std::io::Error;
use std::io::ErrorKind;
use std::io::Write;
use std::io;

pub trait Datum {
    /*
      A datum has a max size of 64kbytes. Thunderhead is not designed
      for very large values; they should be split into multiple values
      instead.
      */
    fn len(&self) -> u16;
    fn write_bytes(&self, w: &mut DataWrite) -> KvResult<()>;
}

pub enum KvResult<T> {
    Success(T),
    // TODO: can we make this StringLike for perf?
    Failure(String),
}

// TODO make this async
// TODO impl return types everywhere
pub trait DataWrite {
    fn write(&mut self, buf: &[u8]) -> KvResult<()>;
}

struct DataWriteWrite<'a, W: 'a + DataWrite + ?Sized> {
    underlying: &'a mut W
}

impl<'a, W: 'a + DataWrite + ?Sized> Write for DataWriteWrite<'a, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.underlying.write(buf) {
            KvResult::Success(_) => Ok(buf.len()),
            KvResult::Failure(s) => Err(Error::new(ErrorKind::Other, s)),
        }
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

// TODO: need a trait to 'async flush' the data write.
pub fn datawrite_write<'a>(w: &'a mut DataWrite) -> impl Write + 'a {
    DataWriteWrite { underlying: w }
}

// TODO all these should be generic
pub trait KvDoc {
    fn write(&self, r: &KvSink) -> KvResult<()>;
}

pub trait KvSink {
    fn write(&self, k: &Datum, v: &Datum) -> KvResult<()>;
}

pub trait KvSource {
    type D: Datum;
    fn read(&self, k: &Datum) -> KvResult<Self::D>;
}

pub trait KvCursor {
    fn get(&self) -> KvResult<&Datum>;
    fn next(self) -> KvCursor;
}

pub trait KvStream {
    fn cursor(&self) -> KvCursor;
}

pub trait SnapshotStamp {
    // TODO extend datum. This is just an impl of Datum
    // that's type-safed. Might be better as a struct.
}

pub trait SnapshotStore<'a> {
    // TODO: persistent snapshot cursors.
    // TODO: what is the correct use of &mut self?

    // These are not reference-counted. Use cases interested in
    // tracking reference counts (or other snapshot metadata)
    // must do so by separate means.

    // TODO: instead, use rust safety to eliminate use-after-free
    // for snapshots...?
    type S: KvSource;
    type Stamp: SnapshotStamp;
    fn open(&mut self) -> Self::Stamp;
    fn close(&mut self, stamp: &Self::Stamp);
    //fn diff(&self, &prev: SnapshotStamp) -> KvStream;
    // TODO make this safer? what is the semantics of a removed snapshot?
    fn snap(&'a self, stamp: &'a Self::Stamp) -> Option<Self::S>;
}

// TODO: Ephemeral kv sink. Good perf for event model POC.
// TODO: permanent kv sink, for obvious reasons. Can we mmap the ephem
// into the permanent?

