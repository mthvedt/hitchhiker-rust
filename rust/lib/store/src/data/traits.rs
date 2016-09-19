use std::borrow::Borrow;
use std::borrow::BorrowMut;

use super::slice::*;

// /// A Datum is something convertable into byte strings, in various ways.
pub trait DataWrite {
    type Result;
    fn write(self, buf: &[u8]) -> Self::Result;
}

pub trait Datum {
	fn len(&self) -> u16;
	fn write_bytes<W: DataWrite>(&self, w: W) -> W::Result;

	type Stream: IntoIterator<Item = u8>;
	type StreamRef: Borrow<Self::Stream>;

	/// Expose this Datum as an iterable stream of bytes.
	/// In general, bounded by the Datum's lifetime. The stream may consume resources
	/// (memory, pinned pages, &c).
	fn as_stream(&self) -> Self::StreamRef;
}

// TODO do we want this given we have iter? TODO should this be a method?
pub fn box_copy<D: Datum>(datum: &D) -> Box<[u8]> {
	// Assuming the optimizer will get rid of extra instructions here, since the
	// only heap allocation is the boxed slice itself.
	let mut r = Vec::with_capacity(datum.len() as usize);
	datum.write_bytes(ByteDataWrite { v: r.borrow_mut() });
	r.into_boxed_slice()
}

// TODO can we make this an anon type?
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

impl<'a> IntoDatum for &'a String {
	type D = <&'a [u8] as IntoDatum>::D;

	fn to_datum(self) -> Self::D {
		self.as_bytes().to_datum()
	}
}
