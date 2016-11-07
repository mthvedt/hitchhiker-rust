use std::io;

use alloc::Scoped;
use data::Range;

use tdfuture::FutureResult;

use futures::Future;
use futures::stream::Stream;

// TODO: need a TdError mechanism.

/// N.B.: We would ideally like T to be an associated type, not a generic type.
/// However, this makes Rust's constraint checker go nuts once we get subtraits (KvSource, KvSink).
/// In particular, it starts demanding manual constraints for GetValue everywhere, even though
/// those constraints should be inferable.
pub trait Source<T: ?Sized> {
	type GetValue: Scoped<T>;
    type Get: Future<Item = Self::GetValue, Error = io::Error>;

    // TODO: should we pass in context? why or why not?
    /// Get a value from this KvSource.
    fn get<K: Scoped<[u8]>>(&mut self, k: K) -> FutureResult<Self::Get>;

    // TODO: StreamResult?
    // type GetMany: Stream<Item = Self::GetValue, Error = io::Error>;
    // fn get_many<K: Scoped<[u8]>, I: IntoIterator<Item = K>>(&mut self, i: I) -> Self::GetMany;

    // type GetRange: Stream<Item = Self::GetValue, Error = io::Error>;
    // fn get_range<K: Scoped<[u8]>>(&mut self, range: Range) -> Self::GetRange;

    /// Note: we have this return Self. Ideally, we would like to retain the ability
    /// to return different types of subtrees; however, this makes Rust's constraint checker
    /// interact poorly with subtraits (like Sink, KvSource, KvSink...) See trait-level docs.
    fn subtree<K: Scoped<[u8]>>(&mut self, k: K) -> Self;

    /// Note: we have this return Self. Ideally, we would like to retain the ability
    /// to return different types of subrange trees; however, this makes Rust's constraint checker
    /// interact poorly with subtraits (like Sink, KvSource, KvSink...) See trait-level docs.
    fn subrange(&mut self, range: Range) -> Self;
}

pub trait Sink<T: ?Sized>: Source<T> {
    type PutSmall: Future<Item = (), Error = io::Error>;

    /// The max size of a value in this KVSource
    fn max_value_size(&self) -> u64;

    /// Put a small value in the KvSink. For large values, one should use an insert stream (not implemented).
    ///
    /// Right now, a strict definition of 'small' is not enforced. A small value is any that can reasonably
    /// fit in an in-memory slice.
    ///
    /// Not that we don't have put_many or put_range. This use case should be handled
    fn put_small<K: Scoped<[u8]>, V: Scoped<T>>(&mut self, k: K, v: V) -> FutureResult<Self::PutSmall>;
}

pub trait KvSource: Source<[u8]> {}

pub trait KvSink: Sink<[u8]> + KvSource {}
