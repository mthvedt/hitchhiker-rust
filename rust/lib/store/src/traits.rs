use alloc::Scoped;
use data::Range;

use tdfuture::FutureResult;

use futures::Future;
use futures::stream::Stream;

pub trait KvCommon {

    /// The max size of a value in this KVSource
    fn max_value_size(&self) -> u64;

    type Subtree;

    fn subtree(&mut self, k: Box<[u8]>) -> Self::Subtree;

    type Subrange;

    fn subrange(&mut self, range: Range) -> Self::Subrange;
}

pub trait KvSource: KvCommon {
	type GetValue: Scoped<[u8]>;
    type Error;


    /// Get a value from this KvSource.

    type Get: Future<Item = Self::GetValue, Error = Self::Error>;
    // TODO: should we pass in context? why or why not?
    fn get<K: Scoped<[u8]>>(&mut self, k: K) -> FutureResult<Self::Get>;

    // TODO: StreamResult?
    type GetMany: Stream<Item = Self::GetValue, Error = Self::Error>;
    fn get_many<K: Scoped<[u8]>, I: IntoIterator<Item = K>>(&mut self, i: I) -> Self::GetMany;

    type GetRange: Stream<Item = Self::GetValue, Error = Self::Error>;
    fn get_range<K: Scoped<[u8]>>(&mut self, range: Range) -> Self::GetRange;
}

pub trait KvSink: KvCommon {
    type PutSmall: Future<Item = (), Error = Self::Error>;
    type Error;

    /// Put a small value in the KvSink. For large values, one should use an insert stream (not implemented).
    ///
    /// Right now, a strict definition of 'small' is not enforced. A small value is any that can reasonably
    /// fit in an in-memory slice.
    ///
    /// Not that we don't have put_many or put_range. This use case should be handled
    fn put_small<K: Scoped<[u8]>, V: Scoped<[u8]>>(&mut self, k: K, v: V) -> FutureResult<Self::PutSmall>;
}
