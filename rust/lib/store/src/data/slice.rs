use std::convert::TryFrom;

use super::traits::*;

// TODO what should be public here?
// TODO: IntoDatum trait, don't publish SliceDatum
pub struct SliceDatum<'a> {
    data: &'a [u8],
}

impl<'a> SliceDatum<'a> {
    pub fn new(slice: &'a [u8]) -> SliceDatum<'a> {
        // TODO don't panic
        u16::try_from(slice.len()).unwrap();
        SliceDatum { data: slice }
    }
}

/// Necessary because the type of &[u8].into_iter() is &u8, not u8.
pub struct SliceDatumIterator<'a> {
    wrapped: <&'a [u8] as IntoIterator>::IntoIter,
}

impl<'a> SliceDatumIterator<'a> {
    fn new(data: &'a [u8]) -> SliceDatumIterator<'a> {
        SliceDatumIterator { wrapped: data.iter(), }
    }
}

impl<'a> Iterator for SliceDatumIterator<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        self.wrapped.next().map(u8::clone)
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

impl<'a> IntoIterator for &'a SliceDatum<'a> {
    type Item = u8;
    type IntoIter = SliceDatumIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        SliceDatumIterator::new(self.data)
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

impl<'a> IntoIterator for &'a SliceDatumMut<'a> {
    type Item = u8;
    type IntoIter = SliceDatumIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        SliceDatumIterator::new(self.data)
    }
}
