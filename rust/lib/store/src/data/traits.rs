use std::borrow::BorrowMut;

// /// A Datum is something convertable into byte strings, in various ways.
pub trait DataWrite {
    type Result;
    fn write(self, buf: &[u8]) -> Self::Result;
}

pub trait Datum {
	fn len(&self) -> u16;
	fn write_bytes<W: DataWrite>(&self, w: W) -> W::Result;

	// TODO
	// type Slice: Borrow<[u8]>;
	// fn load(&'a self) -> Slice;

    fn box_copy(&self) -> Box<[u8]> {
		// Assuming the optimizer will get rid of extra instructions here, since the
		// only heap allocation is the boxed slice itself.
		let mut r = Vec::with_capacity(self.len() as usize);
    	self.write_bytes(ByteDataWrite { v: r.borrow_mut() });
    	r.into_boxed_slice()
    }
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
