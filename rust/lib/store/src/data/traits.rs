use std::borrow::*;

use super::slice::*;

// /// A key is a key for a PersistentTree. It has three properties:
// ///
// /// 1. It can be passed by value quickly.
// ///
// /// 2. It yields fast iterators.
// ///
// /// 3. It can be either moved or quickly cloned into a slice of bytes.
// ///
// /// Boxes (passed by value) fulfill the requirements for a Key, as well as any refernce to a Borrow<[u8]>.
// pub trait Key {
// 	type AsIntoIter: IntoIterator<[u8]>;

// 	fn byte_iter(&self) -> Self::AsIntoIter;

// 	fn into_box(self) -> Box<[u8]>;
// }

// TODO: key is obsolete. Need to use arena bytes model.
pub trait Key {
	fn bytes(&self) -> &[u8];
}

/// Note that this automatically makes any Borrow<[u8]> a Datum, too.
impl<B: ?Sized> Key for B where B: Borrow<[u8]> {
	fn bytes(&self) -> &[u8] {
		self.borrow()
	}
}

pub trait DataWrite {
    type Result;
    fn write(self, buf: &[u8]) -> Self::Result;
}

// TODO obsolete. Need to use arena bytes model or direct disk access model.
pub trait Datum {
	fn len(&self) -> usize;
	// TODO should yield future; both the in and out can be a stream.
	// An 'AndThen' future? what's the overhead?
	fn write_bytes<W: DataWrite>(&self, w: W) -> W::Result;

	/// Try to avoid using this in production.
	// TODO: maybe put into testlib...
	fn box_copy(&self) -> Box<[u8]> {
		// Assuming the optimizer will get rid of extra instructions here, since the
		// only heap allocation is the boxed slice itself.
		let mut r = Vec::with_capacity(self.len() as usize);
		unsafe { r.set_len(self.len() as usize); }
		self.write_bytes(ByteDataWrite { v: r.borrow_mut() });
		r.into_boxed_slice()
	}
}

impl<K> Datum for K where K: Key {
	fn len(&self) -> usize {
		self.bytes().len()
	}

	fn write_bytes<W: DataWrite>(&self, w: W) -> W::Result {
		w.write(self.bytes())
	}
}

// TODO obsolete
struct ByteDataWrite<'a> {
	v: &'a mut [u8],
}

impl<'a> DataWrite for ByteDataWrite<'a> {
	type Result = ();

	fn write(self, buf: &[u8]) -> Self::Result {
		// TODO: safety
		self.v.clone_from_slice(buf);
		()
	}
}

// TODO: we shouldn't need this.
pub trait IntoDatum {
	/// The datum type. In general, this will be bounded by the IntoDatum's lifetime.
	type D: Datum;
	fn into_datum(self) -> Self::D;
}

impl<'a> IntoDatum for &'a [u8] {
	type D = SliceDatum<'a>;

	fn into_datum(self) -> Self::D {
		SliceDatum::new(self)
	}
}

impl<'a> IntoDatum for &'a str {
	type D = <&'a [u8] as IntoDatum>::D;

	fn into_datum(self) -> Self::D {
		self.as_bytes().into_datum()
	}
}

impl<'a> IntoDatum for &'a String {
	type D = <&'a [u8] as IntoDatum>::D;

	fn into_datum(self) -> Self::D {
		self.as_bytes().into_datum()
	}
}
