use std::io;
use std::marker::PhantomData;
use std::mem;

use futures::{Async, Future, Map, Poll};

use alloc::Scoped;

// /// A closure that transforms the output of a Future.
// ///
// /// This exists because future::map is unsuitable for associated types.
// pub trait FutureMap {
//     type Input;
//     type Output;
//     type Error;

//     fn apply(&mut self, i: Self::Input) -> Result<Self::Output, Self::Error>;
// }

// pub struct MapFuture<F: Future, FM: FutureMap<Input = F::Item, Error = F::Error>> {
//     first: F,
//     second: FM,
// }

// impl<F: Future, FM: FutureMap<Input = F::Item, Error = F::Error>> MapFuture<F, FM> {
//     pub fn new(future: F, futuremap: FM) -> Self {
//         MapFuture {
//             first: future,
//             second: futuremap,
//         }
//     }
// }

// impl<F: Future, FM: FutureMap<Input = F::Item, Error = F::Error>> Future for MapFuture<F, FM> {
//     type Item = FM::Output;
//     type Error = F::Error;

//     fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
//         match self.first.poll() {
//             Ok(Async::Ready(x)) => match self.second.apply(x) {
//                 Ok(y) => Ok(Async::Ready(y)),
//                 Err(e) => Err(e),
//             },
//             Ok(Async::NotReady) => Ok(Async::NotReady),
//             Err(e) => Err(e),
//         }
//     }
// }

// /// A continuation that accepts an item and returns a Future.
// ///
// /// This exists because future::and_then is unsuitable for associated types.
// pub trait FutureCont {
//     type Input;
//     type Error;
//     type OutputFuture: Future<Error = Self::Error>;

//     fn apply(self, i: Self::Input) -> Self::OutputFuture;
// }

// pub enum AndThenFuture<F: Future, C: FutureCont<Input = F::Item, Error = F::Error>> {
//     First(F, C),
//     Second(C::OutputFuture),
// }

// impl<F: Future, C: FutureCont<Input = F::Item, Error = F::Error>> AndThenFuture<F, C> {
//     pub fn new(future: F, cont: C) -> Self {
//         AndThenFuture::First(future, cont)
//     }

//     fn unwrap_cont(self) -> C {
//         match self {
//             AndThenFuture::First(_, c) => c,
//             _ => panic!("can't unwrap_cont when Second"),
//         }
//     }
// }

// impl<F: Future, C: FutureCont<Input = F::Item, Error = F::Error>> Future for AndThenFuture<F, C> {
//     type Item = <C::OutputFuture as Future>::Item;
//     type Error = C::Error;

//     fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
//         // Get the result if we can. We return immediately unless we are First and our first future is Ready.
//         let first_result = match *self {
//             AndThenFuture::First(ref mut f, _) => match f.poll() {
//                 Ok(Async::Ready(x)) => x,
//                 Ok(Async::NotReady) => return Ok(Async::NotReady),
//                 Err(e) => return Err(e),
//             },
//             AndThenFuture::Second(ref mut f) => return f.poll(),
//         };

//         // Right now we are First and our first future is consumed. We need to use Cont
//         // and replace self with Second(result of Cont(first_result)).
//         let mut self_tmp = unsafe { mem::uninitialized() };
//         mem::swap(self, &mut self_tmp);
//         let cont = self_tmp.unwrap_cont(); // drops self_tmp
//         let mut newself = AndThenFuture::Second(cont.apply(first_result));
//         mem::swap(self, &mut newself);
//         mem::forget(newself);

//         // We are now AndThenFuture::Second. Poll again, to see if we're done.
//         self.poll()
//     }
// }
