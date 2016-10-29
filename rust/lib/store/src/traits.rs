use std::borrow::Borrow;

use alloc::{Alloc, Scoped};

use tdfuture as td;

// TODO: redescribe trees in terms of these two traits
// pub trait KvSource {
//     type Get: td::PartialResult<Item = <Self::Context as Alloc>::Bytes, Context = Self::Context, Error = Self::Error>;
// 	type Context: Alloc;
//     type Error;

//     /// Get a value from this KvSource.
//     fn get(&mut self, ch: <Self::Context as Alloc>::Handle, k: Scoped<[u8]>) -> Self::Get;
// }

// pub trait KvSink {
//     type InsertSmall: td::PartialResult<Item = (), Context = Self::Context, Error = Self::Error>;
//     type Context: Alloc;
// 	type Error;

//     /// Insert a small value into the KvSink. For large values, one should use an insert stream (not implemented).
//     ///
//     /// Right now, a strict definition of 'small' is not enforced. A small value is any that can reasonably
//     /// fit in an in-memory slice.
//     fn insert_small(&mut self,
//         ch: <Self::Context as Alloc>::Handle,
//         k: <Self::Context as Alloc>::Scoped<[u8]>,
//         v: <Self::Context as Alloc>::Scoped<[u8]>)
//     -> Self::InsertSmall;
// }
