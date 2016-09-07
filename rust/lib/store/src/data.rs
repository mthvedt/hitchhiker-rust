use std::convert::TryFrom;
use byteorder::*;

use traits::*;

pub struct SliceDatum<'a> {
    data: &'a [u8],
}

impl<'a> SliceDatum<'a> {
    pub fn new(slice: &'a [u8]) -> impl Datum + 'a {
        // TODO don't panic
        u16::try_from(slice.len()).unwrap();
        SliceDatum { data: slice }
    }
}

impl<'a> Datum for SliceDatum<'a> {
    fn len(&self) -> u16 {
        u16::try_from(self.data.len()).unwrap()
    }

    fn write_bytes(&self, w: &mut DataWrite) -> KvResult<()> {
        w.write(self.data)
    }
}

pub struct SliceDatumMut<'a> {
    data: &'a mut [u8],
}

impl<'a> SliceDatumMut<'a> {
    pub fn new(slice: &'a mut [u8]) -> SliceDatumMut<'a> {
        // TODO don't panic
        u16::try_from(slice.len()).unwrap();
        SliceDatumMut { data: slice }
    }

    pub fn unwrap(&mut self) -> &mut [u8] {
        self.data
    }
}

impl<'a> Datum for SliceDatumMut<'a> {
    fn len(&self) -> u16 {
        u16::try_from(self.data.len()).unwrap()
    }

    fn write_bytes(&self, w: &mut DataWrite) -> KvResult<()> {
        w.write(self.data)
    }
}

// TODO: hide some of these details
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

    fn write_bytes(&self, w: &mut DataWrite) -> KvResult<()> {
        match datawrite_write(w).write_u64::<LittleEndian>(self.data) {
            Ok(_) => KvResult::Success(()),
            // TODO don't panic
            Err(e) => panic!(e),
        }
    }
}

// TODO: snapshot struct, hide counterdatum64, hide implementation details
// in general
