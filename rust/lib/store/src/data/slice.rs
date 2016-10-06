use super::slicerc::RcSlice;

use std::borrow::Borrow;

use super::traits::*;
// TODO rename this lib
// TODO is value a good name for datum?
#[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ByteBox {
    data: Box<[u8]>,
}

impl ByteBox {
    // TODO: a 'ToBox' trait for Key
    pub fn new<B: Borrow<[u8]>>(bytes: B) -> ByteBox {
        ByteBox {
            // TODO size check
            data: SliceDatum::new(bytes.borrow()).box_copy(),
        }
    }

    pub fn from_key<K: Key + ?Sized>(k: &K) -> ByteBox {
        Self::new(k.bytes())
    }

    pub fn from_value<V: Datum>(v: &V) -> ByteBox {
        ByteBox {
            data: v.box_copy(),
        }
    }
}

impl Borrow<[u8]> for ByteBox {
    fn borrow(&self) -> &[u8] {
        self.data.borrow()
    }
}

#[derive(Clone)]
pub struct ByteRc {
    data: RcSlice<u8>,
}

impl ByteRc {
    pub fn new<B: Borrow<[u8]>>(bytes: B) -> ByteRc {
        Self::from_value(&SliceDatum::new(bytes.borrow()))
    }

    pub fn from_key<K: Key + ?Sized>(k: &K) -> ByteRc {
        Self::new(k.bytes())
    }

    pub fn from_value<V: Datum>(v: &V) -> ByteRc {
        ByteRc {
            data: RcSlice::new(v.box_copy()),
        }
    }
}

impl Borrow<[u8]> for ByteRc {
    fn borrow(&self) -> &[u8] {
        self.data.borrow()
    }
}

// TODO what should be public here?
// TODO impl Key not Datum
#[derive(PartialEq, Eq, Hash)]
pub struct SliceDatum<'a> {
    data: &'a [u8],
}

impl<'a> SliceDatum<'a> {
    pub fn new(slice: &'a [u8]) -> SliceDatum<'a> {
        // TODO don't panic
        // u16::try_from(slice.len()).unwrap();
        SliceDatum { data: slice }
    }
}

// TODO do we need these traits still?
/// Necessary because the type of &[u8].into_iter() is &u8, not u8.
pub struct SliceDatumIterator<'a> {
    wrapped: <&'a [u8] as IntoIterator>::IntoIter,
}

impl<'a> SliceDatumIterator<'a> {
    fn new(data: &'a [u8]) -> SliceDatumIterator<'a> {
        SliceDatumIterator { wrapped: data.iter(), }
    }
}

// TODO are these used?
impl<'a> Iterator for SliceDatumIterator<'a> {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        self.wrapped.next().map(u8::clone)
    }
}

impl<'a> Datum for SliceDatum<'a> {
    fn len(&self) -> usize {
        self.data.len()
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

// #[derive(PartialEq, Eq, Hash)]
// pub struct SliceDatumMut<'a> {
//     data: &'a mut [u8],
// }

// impl<'a> SliceDatumMut<'a> {
//     pub fn new(slice: &'a mut [u8]) -> SliceDatumMut<'a> {
//         // TODO don't panic
//         // u16::try_from(slice.len()).unwrap();
//         SliceDatumMut { data: slice }
//     }

//     pub fn unwrap(&mut self) -> &mut [u8] {
//         self.data
//     }
// }

// impl<'a> Datum for SliceDatumMut<'a> {
//     fn len(&self) -> usize {
//         self.data.len()
//     }

//     fn write_bytes<W: DataWrite>(&self, w: W) -> W::Result {
//         w.write(self.data)
//     }
// }

// impl<'a> IntoIterator for &'a SliceDatumMut<'a> {
//     type Item = u8;
//     type IntoIter = SliceDatumIterator<'a>;

//     fn into_iter(self) -> Self::IntoIter {
//         SliceDatumIterator::new(self.data)
//     }
// }
