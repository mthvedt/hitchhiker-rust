use std::mem;

use futures::{Async, Future, Poll};

pub enum FutureResult<F: Future> {
    Ok(F::Item),
    Err(F::Error),
    Wait(F),
}

// enum FutureResultInt<Item, Error> {
//     Done(Item),
//     Err(Error),
//     Wait(BounceFuture<Item = Item, Error = Error>),
// }

// /// A partial computation. It can represent a concrete value, an error,
// /// a Future, or something else that can be passed to a completable computation.
// pub trait Partial {
//     type Item;
//     type Context;
//     type Error;

//     fn complete<C: Continuation>(self, cont: C) -> CompleteResult<Self::Error> where
//     Cont: FnOnce(Item, Context) -> CompleteResult<Self::Error> + Send + 'static;
// }

// pub trait Continuation {
//     type Input;
//     type Context;
//     type Error;

//     fn continue<ErrorHandler>(self, input: Self::Input) -> Option<Error>;
// }

// /// The result of a computation that can finish immediately or yield a Partial.
// pub enum MultiResult<F: Future> {
//     Ok(F::Item),
//     Err(F::Error),
//     Wait(F),
// }

// impl<F: PartialFuture> Partial for MultiResult<F> {
//     type Item = F::Item;
//     type Context = F::Context;
//     type Error = F::Error;

//     fn complete<Cont>(self, cont: Cont) -> FutureResult<Self::Error> where
//     Cont: FnOnce(Item, Context) -> FutureResult<Self::Error> + Send + 'static {
//         match self {
//             Ok(item) => cont(item, context),
//             Err(err) => CompleteResult::Error(err),
//             Wait(f) => CompleteResult::Task(f.chain(cont)),
//         }
//     }
// }

// /// A future task in Thunderhead's futures system.
// /// that executes with a context to return an item or an error.
// /// This often is a wrapped future or dependent on a wrapped future.
// pub trait PartialTask {
//     type Context;
//     type Item;
//     type Error;

//     /// Polls this future. Must have an active futures-rs task.

//     // TODO: assert the above.
//     fn exec(&mut self) -> ExecStatus<Self::Item, Self::Error>;

//     // /// Chains this future into a CompleteResult.
//     // ///
//     // /// Because this may call poll(), we must have an active futures-rs task.
//     // fn complete<Cont>(self, cont: Cont) -> CompleteResult<Self::Error> where
//     // Cont: FnOnce(Self::Item, Self::Context) -> CompleteResult<Self::Error> + Send + 'static;
// }

// /// Wraps a futures-rs future into a PartialFuture.
// pub struct PartialFutureWrapper<R, C, F: futures::Future<Item = (R, C)> + Send + 'static> {
//     inner: F,
// }

// struct PartialFutureCompletion<F, Cont> where
// F: PartialFuture,
// Cont: FnOnce(F::Item) -> CompleteResult<F::Error> + Send + 'static
// {
//     future: F,
//     cont: Cont,
// }

// impl<F, Cont> futures::Future for PartialFutureCompletion<F, Cont> where
// F: PartialFuture,
// Cont: FnOnce(F::Item) -> CompleteResult<F::Error> + Send + 'static
// {
//     type Item = Option<Future<F::Error>>;
//     type Error = F::Error;

//     fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
//         match self.future.poll() {
//             Poll::Ready(item) => match (self.cont)(item) {
//                 CompleteResult::Ok => Ok(Async::Ready(None)),
//                 CompleteResult::Err(err) => Err(err),
//                 CompleteResult::Wait(future) => Ok(Async::Ready(Some(future))),
//             },
//             Poll::Err(err) => Err(err),
//             Poll::Wait => Ok(Async::NotReady),
//         }
//     }
// }

// impl<F: futures::Future + Send + 'static> PartialFuture for PartialFutureWrapper<F> {
//     type Item = F::Item;
//     type Error = F::Error;

//     fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
//         Poll::wrap_futures_rs(self.inner.poll())
//     }

//     fn complete<Cont>(self, cont: Cont) -> CompleteResult<Self::Error> where
//     Cont: FnOnce(Self::Item) -> CompleteResult<Self::Error> + Send + 'static,
//     {
//         match self.poll() {
//             Poll::Ready(item) => cont(item),
//             Poll::Err(err) => CompleteResult::Err(err),
//             Poll::Wait => CompleteResult::Wait(Future {
//                 inner: self.inner.map(|item| {
//                     match cont(item) {

//                     }
//                 }).boxed(),
//             }),
//         }
//     }
// }

// pub trait PartialResult {
//     type Item;
//     type Error;

//     fn complete<Cont>(self, cont: Cont) -> CompleteResult<Error> where
//     Cont: FnOnce(Item) -> CompleteResult<Error> + Send + 'static;
// }

// /// The result of a continuation which may or may not return a PartialFuture.
// pub enum PartialFutureResult<F: PartialFuture> {
//     Ok(F::Item),
//     Err(F::Error),
//     Wait(F),
// }

// impl<F: PartialFuture> PartialResult for PartialFutureResult<F> {
//     type Item = F::Item;
//     type Error = F::Error;

//     fn complete<Cont>(self, cont: Cont) -> CompleteResult<F::Error> where
//     Cont: FnOnce(F::Item) -> CompleteResult<F::Error> + Send + 'static,
//     {
//         match self {
//             PartialFutureResult::Ok(i) => cont(i),
//             PartialFutureResult::Err(e) => CompleteResult::Err(e),
//             PartialFutureResult::Wait(f) => f.complete(cont),
//         }
//     }
// }

// /// The result of a continuation which may either be a Future or nothing.
// pub enum CompleteResult<E> {
//     Ok,
//     Err(E),
//     Wait(Future<E>),
// }

// /// An executable task in Thunderhead's futures system.
// ///
// /// The design goal is to be the fastest possible representation for an enqueued future,
// /// which is a single closure with function pointer stored inline.
// pub struct FutureTask {
// 	inner: Box<futures::Future<Item = Option<Future<E>>, Error = E>>,
// }

// impl<E> Future<E> {
// 	/// Executes a future. Usually you want to do this in an event loop.
// 	pub fn poll(&mut self) -> Poll<(), E> {
// 		use futures::Async;

// 		loop {
// 			let newbox = match self.inner.as_mut().poll() {
// 				Ok(Async::NotReady) => return Poll::Wait,
// 				Ok(Async::Ready(Some(f))) => f.inner, // The only case where we don't return
//                 Ok(Async::Ready(None)) => return Poll::Ready(()),
// 				Err(e) => return Poll::Err(e),
// 			};

// 			// We have a new future. Make it ours, and poll it in the next iteration of the loop...
// 			mem::replace(&mut self.inner, newbox);
// 		}
// 	}
// }
