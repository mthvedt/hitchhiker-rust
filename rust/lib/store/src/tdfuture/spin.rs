use std::{mem, ptr};

use futures::{Async, Future, Poll};

/// The result of a SpinLambda, which can yield a SpinFuture.
pub enum SpinResult<S: SpinLambda> {
	Ok(S::Item),
	Err(S::Error),
	Spin(SpinFuture<S>),
}

impl<S: SpinLambda> SpinResult<S> {
	pub fn ok(item: S::Item) -> Self {
		SpinResult::Ok(item)
	}

	pub fn err(err: S::Error) -> Self {
		SpinResult::Err(err)
	}

	/// Combines a future with a SpinLambda to form a SpinFuture wrapped in a SpinResult.
	pub fn chain(in_future: S::BlockingFuture, continuation: S) -> Self {
		SpinResult::Spin(SpinFuture::chain(in_future, continuation))
	}

	/// Turns this SpinReslt into a Future, so it can be combined with future combinators.
	/// Polling the resultant Future yields either the contained Item, the contained Err,
	/// or polls the contained Future.
	pub fn to_future(self) -> SpinResultFuture<S> {
		match self {
			SpinResult::Ok(item) => SpinResultFuture::Ok(item),
			SpinResult::Err(err) => SpinResultFuture::Err(err),
			SpinResult::Spin(f) => SpinResultFuture::Spin(f),
		}
	}
}

/// A future version of SpinResult. See `SpinResult::to_future(self)`.
pub enum SpinResultFuture<S: SpinLambda> {
	Ok(S::Item),
	Err(S::Error),
	Spin(SpinFuture<S>),
	// The wrinkle is we need a way to safely move the Ok and Err states.
	Consumed,
}

impl<S: SpinLambda> Future for SpinResultFuture<S> {
	type Item = S::Item;
	type Error = S::Error;

	fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
		let mut oldself = SpinResultFuture::Consumed;
		mem::swap(self, &mut oldself);

		match oldself {
			SpinResultFuture::Ok(item) => Ok(Async::Ready(item)),
			SpinResultFuture::Err(err) => Err(err),
			SpinResultFuture::Spin(mut f) => {
				let r = f.poll();
				*self = SpinResultFuture::Spin(f);
				r
			}
			SpinResultFuture::Consumed => panic!("Cannot poll a complete future twice"),
		}
	}
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
/// # CS note: Y not use the Y-combinator (or some other fixed-point combinator)?
///
/// The (SpinLambda, SpinFuture) mechanism allows us to define a recursive Future without
/// complicated types or fixed-point combinator gymnastics, which are difficult to pull off (and understand)
/// elegantly in Rust. At least, that was what I found.
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
	pub fn chain(in_future: S::BlockingFuture, continuation: S) -> Self {
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
