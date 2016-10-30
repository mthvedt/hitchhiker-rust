use std::mem;

use futures::{Async, Future, Poll};

/// The result of a SpinLambda, which can yield a SpinFuture.
pub enum SpinResult<S: SpinLambda> {
	Ok(S::Item),
	Err(S::Error),
	Spin(SpinFuture<S>),
}

/// A closure that can return a SpinResult. In other words, it can return Ok(Self::Item),
/// Error(Self::Error), or return a Future which might be blocked. (It doesn't have to be blocked.)
/// In conjunction with SpinFuture, this can be used to represent a directly recursive Future
/// without any extra allocations or transition states.
///
/// # Examples
///
/// Say you had a future which searches the nodes of an on-disk B-tree for a value. It is unknown
/// how many nodes are on disk or in memory. When it encounters an element on disk,
/// it must return a future. We can have a `SpinLambda` named `SearchLambda` that returns a tree value
/// when it finds one, recurses direclty if it encounters an in-memory node,
/// or returns a SpinFuture depending on a Future load from disk if it encounters an on-disk node.
///
/// # The name spin_future
///
/// It is called so because the SpinFuture can 'spin' by returning a copy of itself.
/// The returned future then replaces its internal copy. In this way an indefinite series of futures
/// is represented without allocating or changing internal states.
///
/// # Type theory note: Y not use the Y-combinator (or some other fixed-point combinator)?
///
/// The (SpinLambda, SpinFuture) mechanism allows us to define a recursive Future without
/// complicated types or fixed-point combinator gymnastics. The drawback is implementations of SpinFuture
/// must go through SpinLambda, which is a little more verbose than a closure+combinator combo.
pub trait SpinLambda: Sized {
	/// The blocked future type for this SpinLambda.
	type BlockingFuture: Future<Error = Self::Error>;
	type Item;
	type Error;

	fn spin(&mut self, i: <Self::BlockingFuture as Future>::Item) -> SpinResult<Self>;
}

/// Wraps a SpinLambda into a Future.
pub struct SpinFuture<S: SpinLambda> {
	in_future: S::BlockingFuture,
	spin_lambda: S,
}

impl<S: SpinLambda> SpinFuture<S> {
	/// Combines a future with a SpinLambda to form a SpinFuture.
	pub fn chain(in_future: S::BlockingFuture, continuation: S) -> SpinFuture<S> {
		SpinFuture {
			in_future: in_future,
			spin_lambda: continuation,
		}
	}
}

impl<S: SpinLambda> Future for SpinFuture<S> {
	type Item = S::Item;
	type Error = S::Error;

	fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
		// Let's try spinning! That's a good trick!
		// We 'spin' in_future and spin_lambda until we get something that blocks.
		loop {
			let mut newself = match self.in_future.poll() {
				Ok(Async::NotReady) => return Ok(Async::NotReady),
				Ok(Async::Ready(item)) => match self.spin_lambda.spin(item) {
					SpinResult::Ok(item) => return Ok(Async::Ready(item)),
					SpinResult::Err(err) => return Err(err),
					// The only branch which does not return immediately.
					SpinResult::Spin(s) => s,
				},
				Err(err) => return Err(err),
			};

			// We only reach here if spin_lambda returned Spin. We loop because
			// we have to poll the new in_future.
			mem::swap(self, &mut newself);
		}
	}
}

// /// Takes a future that can return an item or another copy of itself,
// /// and turns it into a future which just returns the item.
// /// This can be used to wrap an indefinite loop of futures into one future.
// ///
// pub fn spin_future<F, Item>(future: F) -> SpinFuture<F, Item> where F: Future<Item = Result<Item, F>> {
// 	SpinFuture {
// 		inner: future,
// 	}
// }

// /// Helper to turn a function into a spin_future.
// ///
// /// This turns a recursive future-based function and folds it into a single future with bounded internal state.
// /// Useful for turning indefinite recursion into an O(1)-sized future.
// ///
// /// # Examples
// pub fn spin_future<F, Lambda, EndItem>(start_item: &Item, lambda: L) -> SpinResult<Item, F> where
// F: Future<Item = Result<Item, F>>,
// L: Fn(&Item) -> SpinResult<EndItem, F>,
// {
// 	match lambda(item) {
// 		Ok(end_item) => Ok(end_item),
// 		Err(err) => Err(err),
// 		Wait(f) => Wait(f)
// 	}
// }

// // pub fn spinify<F, R>(lambda: L) -> SpinFuture<F, R> where
// // L: FnOnce() -> FutureResult<F, R>,

// impl<F, EndItem> Future for SpinFuture<F, EndItem> where F: Future<Item = Result<EndItem, F>> {
//     type Item = EndItem;
//     type Error = F::Error;

//     fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
//         match self.inner.poll() {
// 	        Ok(Async::Ready(Ok(r))) => Ok(Async::Ready(r)),
// 	        Ok(Async::Ready(Err(new_inner))) => {
// 	            // The point of a spin future--replace self with the next one in the chain
// 	            self.inner = new_inner;
// 	            Ok(Async::NotReady)
// 	        }
// 	        Ok(Async::NotReady) => Ok(Async::NotReady),
// 	        Err(err) => Err(err),
//         }
//     }
// }
