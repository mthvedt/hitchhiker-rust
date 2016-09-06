use std::convert::TryFrom;
use byteorder::*;

use traits::*;

// TODO: what to hide?
pub struct SliceDatum<'a> {
    data: &'a [u8],
}

impl<'a> Datum for SliceDatum<'a> {
    fn len(&self) -> u16 {
        u16::try_from(self.data.len()).unwrap()
    }

    fn write_bytes(&self, w: &mut DataWrite) -> KvResult<()> {
        w.write(self.data)
    }

    /*
    fn to_slice(&self) -> &[i8] {
        data
    }
    */
}

pub fn slice_datum<'a>(slice: &'a [u8]) -> impl Datum + 'a {
    // TODO don't panic
    u16::try_from(slice.len()).unwrap();
    SliceDatum { data: slice }
}

struct SliceDatumMut<'a> {
    data: &'a mut [u8],
}

impl<'a> Datum for SliceDatumMut<'a> {
    fn len(&self) -> u16 {
        u16::try_from(self.data.len()).unwrap()
    }

    fn write_bytes(&self, w: &mut DataWrite) -> KvResult<()> {
        w.write(self.data)
    }

    /*
    fn to_slice(&self) -> &[i8] {
        data
    }

    fn to_slice_mut(&mut self) -> &mut [i8] {
        data
    }
    */
}

pub fn slice_datum_mut<'a>(slice: &'a mut [u8]) -> impl Datum + 'a {
    // TODO don't panic
    u16::try_from(slice.len()).unwrap();
    SliceDatumMut { data: slice }
}

// TODO: hide some of these details
// TODO: consider perf consequences of making this variable-sized
// TODO: if the above, make this u128
#[derive(PartialEq, Eq, Hash, Clone)]
pub struct CounterDatum64 {
    // little-endian
    data: u64,
}

impl CounterDatum64 {
    pub fn new(x: u64) -> CounterDatum64 {
        CounterDatum64 { data: x }
    }

    pub fn inc(&self) -> CounterDatum64 {
        CounterDatum64 { data: self.data + 1 }
    }
}

impl Datum for CounterDatum64 {
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
