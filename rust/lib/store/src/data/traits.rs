use std::borrow::BorrowMut;

use super::slice::*;

pub trait DataWrite {
    type Result;
    fn write(self, buf: &[u8]) -> Self::Result;
}

pub trait Datum {
	fn len(&self) -> u16;
	// TODO should yield future; both the in and out can be a stream.
	// An 'AndThen' future? what's the overhead?
	fn write_bytes<W: DataWrite>(&self, w: W) -> W::Result;

	// TODO do we want this given we have iter? TODO should this be a method?
	fn box_copy(&self) -> Box<[u8]> {
		// Assuming the optimizer will get rid of extra instructions here, since the
		// only heap allocation is the boxed slice itself.
		let mut r = Vec::with_capacity(self.len() as usize);
		unsafe { r.set_len(self.len() as usize); }
		self.write_bytes(ByteDataWrite { v: r.borrow_mut() });
		r.into_boxed_slice()
	}
}

// TODO can we make this an anon type?
struct ByteDataWrite<'a> {
	v: &'a mut [u8],
}

// TODO need to work on api for datawrite...
impl<'a> DataWrite for ByteDataWrite<'a> {
	type Result = ();

	fn write(self, buf: &[u8]) -> Self::Result {
		// TODO: safety
		self.v.clone_from_slice(buf);
		()
	}
}

pub trait IntoDatum {
	/// The datum type. In general, this will be bounded by the IntoDatum's lifetime.
	type D: Datum;
	fn to_datum(self) -> Self::D;
}

impl<'a> IntoDatum for &'a [u8] {
	type D = SliceDatum<'a>;

	fn to_datum(self) -> Self::D {
		SliceDatum::new(self)
	}
}

impl<'a> IntoDatum for &'a str {
	type D = <&'a [u8] as IntoDatum>::D;

	fn to_datum(self) -> Self::D {
		self.as_bytes().to_datum()
	}
}

impl<'a> IntoDatum for &'a String {
	type D = <&'a [u8] as IntoDatum>::D;

	fn to_datum(self) -> Self::D {
		self.as_bytes().to_datum()
	}
}
