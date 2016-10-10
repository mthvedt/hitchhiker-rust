use std::borrow::Borrow;

use data::{ByteRc, Datum};

#[derive(Clone)]
pub struct Bucket {
	k: ByteRc,
	v: ByteRc,
}

impl Bucket {
	pub fn new<V: Datum>(k: &[u8], v: &V) -> Bucket {
		Bucket {
			k: ByteRc::from_key(k),
			v: ByteRc::from_value(v),
		}
	}
}

#[derive(Clone)]
pub struct BucketPtr {
	v: Option<Bucket>,
}

impl BucketPtr {
	pub fn empty() -> BucketPtr {
		BucketPtr {
			v: None,
		}
	}

	pub fn wrap(b: Bucket) -> BucketPtr {
		BucketPtr { v: Some(b), }
	}

	pub fn unwrap(self) -> Bucket {
		self.v.unwrap()
	}

	pub fn key(&self) -> &[u8] {
		self.v.as_ref().unwrap().k.borrow()
	}

	// TODO: return an address
	pub fn value_address(&self) -> ByteRc {
		self.v.as_ref().unwrap().v.clone()
	}
}
