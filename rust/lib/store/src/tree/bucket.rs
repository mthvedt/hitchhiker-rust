use std::borrow::Borrow;

use data::{ByteRc, Datum};

#[derive(Clone)]
pub struct Bucket {
	pub k: ByteRc,
	pub v: ByteRc,
}

impl Bucket {
	pub fn new<V: Datum>(k: &[u8], v: &V) -> Bucket {
		Bucket {
			// TODO: from_key or from_bytes?
			k: ByteRc::from_key(k),
			v: ByteRc::from_value(v),
		}
	}
}
