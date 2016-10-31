use std::borrow::Borrow;

use alloc::{Scoped};

use tdfuture::FutureResult;

use futures::Future;

pub trait KvSource {
	type GetValue: Scoped<[u8]>;
    type Get: Future<Item = Self::GetValue, Error = Self::Error>;
	type Context;
    type Error;

    /// Get a value from this KvSource.
    fn get<K: Scoped<[u8]>, C: AsRef<Self::Context>>(&mut self, ctx: Self::Context, k: K) -> FutureResult<Self::Get>;
}

pub trait KvSink {
    type InsertSmall: Future<Item = (), Error = Self::Error>;
    type Context;
    type Error;

    /// Insert a small value into the KvSink. For large values, one should use an insert stream (not implemented).
    ///
    /// Right now, a strict definition of 'small' is not enforced. A small value is any that can reasonably
    /// fit in an in-memory slice.
    fn insert_small<K: Scoped<[u8]>, V: Scoped<[u8]>, C: AsRef<Self::Context>>(&mut self, ctx: C, k: K, v: V)
    -> FutureResult<Self::InsertSmall>;
}
