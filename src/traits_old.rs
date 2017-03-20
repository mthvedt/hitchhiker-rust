// use std::{self, io};
//
// use futures::Future;
//
// use alloc::Scoped;
// use data::Range;
//
// // TODO improve this.
// // TODO: hide details
// #[derive(Debug)]
// pub enum TreeError {
//     EvalError,
//     // TODO: distinguish by error role. EvalError, DbError &c
//     IoError(io::Error),
//     RuntimeError(String),
// }
//
// impl TreeError {
//     pub fn new_io<E: Into<Box<std::error::Error + Send + Sync>>>(kind: io::ErrorKind, error: E) -> TreeError {
//         io::Error::new(kind, error).into()
//     }
// }
//
// impl From<io::Error> for TreeError {
//     fn from(e: io::Error) -> Self {
//         TreeError::IoError(e)
//     }
// }
//
// /// N.B.: We would ideally like T to be an associated type, not a generic type.
// /// However, this makes Rust's constraint checker go nuts once we get subtraits (KvSource, KvSink).
// /// In particular, it starts demanding manual constraints for T everywhere, even though
// /// those constraints should be inferable.
//
// // TODO: we might want to make keys generic
// // TODO: do we want subtree/subrange to be part of source?
// // TODO: do we want to expose Scoped? why not Borrow?
// // TODO: T should be associated type
// // TODO: can we make KvSource a Scoped source?
// // TODO: can we make subtrees/subranges borrowed? like: subtree(&'b mut self) -> Self<'b>?
// pub trait Source<T: ?Sized + 'static> {
// 	type Get: Scoped<T> + 'static;
//     type GetF: Future<Item = Option<Self::Get>, Error = TreeError> + 'static;
//
//     // TODO: should we pass in context? why or why not?
//     /// Get a value from this KvSource.
//     fn get<K: Scoped<[u8]>>(&mut self, k: K) -> Self::GetF;
//
//     // TODO: StreamResult?
//     // type GetMany: Stream<Item = Self::GetValue, Error = io::Error>;
//     // fn get_many<K: Scoped<[u8]>, I: IntoIterator<Item = K>>(&mut self, i: I) -> Self::GetMany;
//
//     // type GetRange: Stream<Item = Self::GetValue, Error = io::Error>;
//     // fn get_range<K: Scoped<[u8]>>(&mut self, range: Range) -> Self::GetRange;
//
//     /// Note: we have this return Self. Ideally, we would like to retain the ability
//     /// to return different types of subtrees; however, this makes Rust's constraint checker
//     /// interact poorly with subtraits (like Sink, KvSource, KvSink...) See trait-level docs.
//     fn subtree<K: Scoped<[u8]>>(&mut self, k: K) -> Self;
//
//     /// Note: we have this return Self. Ideally, we would like to retain the ability
//     /// to return different types of subrange trees; however, this makes Rust's constraint checker
//     /// interact poorly with subtraits (like Sink, KvSource, KvSink...) See trait-level docs.
//     fn subrange(&mut self, range: Range) -> Self;
// }
//
// pub trait Sink<T: ?Sized + 'static>: Source<T> {
//     type PutF: Future<Item = (), Error = TreeError> + 'static;
//
//     /// The max size of a value in this KVSource
//
//     // TODO: this should be a constant? At the very least, max_value_size should be a property
//     // of a Lens.
//     fn max_value_size(&self) -> u64;
//
//     /// Put a small value in the KvSink. For large values, one should use an insert stream (not implemented).
//     ///
//     /// Right now, a strict definition of 'small' is not enforced. A small value is any that can reasonably
//     /// fit in an in-memory slice.
//     ///
//     /// Not that we don't have put_many or put_range. This use case should be handled
//     fn put_small<K: Scoped<[u8]>, V: Scoped<T>>(&mut self, k: K, v: V) -> Self::PutF;
// }
//
// // TODO: Sink doesn't need to implement Source
//
// pub trait KvSource: Source<[u8]> {}
// impl<S> KvSource for S where S: Source<[u8]> {}
//
// pub trait KvSink: Sink<[u8]> + KvSource {}
// impl<S> KvSink for S where S: Sink<[u8]> {}
//
// pub trait StringSource: Source<str> {}
// impl<S> StringSource for S where S: Source<str> {}
//
// pub trait StringSink: Sink<str> + StringSource {}
// impl<S> StringSink for S where S: Sink<str> {}
