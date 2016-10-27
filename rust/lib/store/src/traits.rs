use std::borrow::Borrow;

use tdfuture as td;

// TODO: redescribe trees in terms of these two traits
pub trait KvSource {
	type Error;

    fn get<K: Borrow<[u8]>>(&mut self, k: &K) -> td::Result<Box<[u8]>, Self::Error>;
}

pub trait KvSink {
	type Error;

    fn insert<K: Borrow<[u8]>, V: Borrow<[u8]>>(&mut self, k: &K, v: &V) -> td::Result<(), Self::Error>;
}
