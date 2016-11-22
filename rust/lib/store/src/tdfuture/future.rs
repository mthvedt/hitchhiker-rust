use std::io;
use std::marker::PhantomData;
use std::mem;

use futures::{Async, Future, Map, Poll};

use alloc::Scoped;

// trait GlobalContext {
//     fn kill(&self);

//     fn is_live(&self) -> bool;
// }

// trait Context {
//     type GlobalContext: Context;

//     fn global_context(&self) -> Self::GlobalContext;
// }

// enum Poll<Item> {
//     Ok(Item),
//     Err(io::Error),
//     Fatal(io::Error),
//     Wait,
// }

// /// A Thunderhead future.
// trait Future<C: Context> {
//     type Item;

//     fn poll(&mut self, c: &mut C) -> Poll<Self::Item>;
// }

// TODO: we don't use these. Right now we are really concerned about JS speed. For Rust speed
// it is probably better to use the cont mechanism.

// /// The result of a fn that may optionally return a future.
// ///
// /// They are not futures by default. The motivation behind this is that we want to continue directly,
// /// instead of using the future mechanism, when a result is ready immediately. Additionally,
// /// we want to use the FutureChain (not yet implemented) mechanism.
// ///
// /// It is easy to adapt
// /// a FutureResult into a FutureResultFuture; because their memory layouts are the same, this is
// /// usually efficient.
// pub enum FutureResult<F: Future> {
//     Ok(F::Item),
//     Err(F::Error),
//     Wait(F),
// }

// impl<F: Future> FutureResult<F> {
//     pub fn map<Item2, FC: FnOnce(F::Item) -> Item2>(self, fc: FC) -> FutureResult<Map<F, FC>> {
//         match self {
//             FutureResult::Ok(x) => FutureResult::Ok((fc)(x)),
//             FutureResult::Err(e) => FutureResult::Err(e),
//             FutureResult::Wait(f) => FutureResult::Wait(f.map(fc)),
//         }
//     }

//     pub fn to_future(self) -> FutureResultFuture<F> {
//         match self {
//             FutureResult::Ok(item) => FutureResultFuture::Ok(item),
//             FutureResult::Err(e) => FutureResultFuture::Err(e),
//             FutureResult::Wait(f) => FutureResultFuture::Wait(f),
//         }
//     }
// }


// /// A future version of FutureResult. See `FutureResult::to_future(self)`.
// pub enum FutureResultFuture<F: Future> {
//     Ok(F::Item),
//     Err(F::Error),
//     Wait(F),
//     /// A consumed FutureResultFuture. This exists so poll can move out of the Ok and Err states.
//     /// Polling a consumed FutureResultFuture is an error.
//     Consumed,
// }

// impl<F: Future> Future for FutureResultFuture<F> {
//     type Item = F::Item;
//     type Error = F::Error;

//     fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
//         let mut oldself = FutureResultFuture::Consumed;
//         mem::swap(self, &mut oldself);

//         match oldself {
//             FutureResultFuture::Ok(item) => Ok(Async::Ready(item)),
//             FutureResultFuture::Err(err) => Err(err),
//             FutureResultFuture::Wait(mut f) => {
//                 let r = f.poll();
//                 // We don't actually need to check what we polled.
//                 // We just keep the future around; it can be polled again if needed.
//                 *self = FutureResultFuture::Wait(f);
//                 r
//             }
//             FutureResultFuture::Consumed => panic!("Cannot poll a complete future twice"),
//         }
//     }
// }

/// A closure that transforms the output of a Future.
///
/// This exists because future::map is unsuitable for associated types.
pub trait FutureMap {
    type Input;
    type Output;
    type Error;

    fn apply(&mut self, i: Self::Input) -> Result<Self::Output, Self::Error>;
}

pub struct MapFuture<F: Future, FM: FutureMap<Input = F::Item, Error = F::Error>> {
    first: F,
    second: FM,
}

impl<F: Future, FM: FutureMap<Input = F::Item, Error = F::Error>> MapFuture<F, FM> {
    pub fn new(future: F, futuremap: FM) -> Self {
        MapFuture {
            first: future,
            second: futuremap,
        }
    }
}

impl<F: Future, FM: FutureMap<Input = F::Item, Error = F::Error>> Future for MapFuture<F, FM> {
    type Item = FM::Output;
    type Error = F::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.first.poll() {
            Ok(Async::Ready(x)) => match self.second.apply(x) {
                Ok(y) => Ok(Async::Ready(y)),
                Err(e) => Err(e),
            },
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err(e),
        }
    }
}

/// A continuation that accepts an item and returns a Future.
///
/// This exists because future::and_then is unsuitable for associated types.
pub trait FutureCont {
    type Input;
    type Error;
    type OutputFuture: Future<Error = Self::Error>;

    fn apply(self, i: Self::Input) -> Self::OutputFuture;
}

pub enum AndThenFuture<F: Future, C: FutureCont<Input = F::Item, Error = F::Error>> {
    First(F, C),
    Second(C::OutputFuture),
}

impl<F: Future, C: FutureCont<Input = F::Item, Error = F::Error>> AndThenFuture<F, C> {
    pub fn new(future: F, cont: C) -> Self {
        AndThenFuture::First(future, cont)
    }

    fn unwrap_cont(self) -> C {
        match self {
            AndThenFuture::First(_, c) => c,
            _ => panic!("can't unwrap_cont when Second"),
        }
    }
}

impl<F: Future, C: FutureCont<Input = F::Item, Error = F::Error>> Future for AndThenFuture<F, C> {
    type Item = <C::OutputFuture as Future>::Item;
    type Error = C::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        // Get the result if we can. We return immediately unless we are First and our first future is Ready.
        let first_result = match *self {
            AndThenFuture::First(ref mut f, _) => match f.poll() {
                Ok(Async::Ready(x)) => x,
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(e) => return Err(e),
            },
            AndThenFuture::Second(ref mut f) => return f.poll(),
        };

        // Right now we are First and our first future is consumed. We need to use Cont
        // and replace self with Second(result of Cont(first_result)).
        let mut self_tmp = unsafe { mem::uninitialized() };
        mem::swap(self, &mut self_tmp);
        let cont = self_tmp.unwrap_cont(); // drops self_tmp
        let mut newself = AndThenFuture::Second(cont.apply(first_result));
        mem::swap(self, &mut newself);
        mem::forget(newself);

        // We are now AndThenFuture::Second. Poll again, to see if we're done.
        self.poll()
    }
}
