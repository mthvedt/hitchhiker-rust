use std::convert::TryFrom;

use super::traits::*;

// TODO what should be public here?
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

// TODO: W or &mut W? Let's go with W--makes it easier to used sized data writes
// TODO: consider api for fixed/variable data writes
// TODO: consider safety checks here
    fn write_bytes<W: DataWrite>(&self, w: W) -> W::Result {
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

    fn write_bytes<W: DataWrite>(&self, w: W) -> W::Result {
        w.write(self.data)
    }
}
