//!
//!
//! # Perf notes
//!
//! Every nontrivial ChainState involves an alloc and copy. For large or frequently interrupted chains,
//! this is a source of inefficiency.
//!
//! It is possible but tedious to implement ChainState with only O(amortized max size)
//! allocations for the entire state; using a large allocation buffer, the overhead becomes trivial.
//! Basically, this is done by keeping a stack representing the chain continuation--but allocating the stack
//! on the heap.
//! See the unfinished work in alloc/stack.
//!
//! # CS notes
//!
//! The astute computer scientist will note that Chains are continuations. If we implement clone for ChainState,
//! we recover the shift/reset mechanism from Scheme or the ContT monad from Haskell.
//! Continuation-passing code is too complex for our use case, however, so this case is not implemented.

use std::io;
use std::mem;

use futures::{Async, Future, Poll};

use chain::{bind, Chain};

// /// An uninstantiable Future. Useful for type gymnastics.
// struct VoidFuture<Item, Error> {
//     phantom: PhantomData<(*const Item, *const Error)>,
// }

// impl<Item, Error> Future for VoidFuture<Item, Error> {
//     type Item = Item;
//     type Error = Error;

//     fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
//         unreachable!()
//     }
// }

/// The result of a fn that may optionally return a future.
///
/// They are not futures by default, but may be turned into a FutureResultFuture.
pub enum FutureResult<F: Future> {
    Ok(F::Item),
    Err(F::Error),
    Wait(F),
}

impl<F: Future> FutureResult<F> {
    // pub fn map<Item2, FC: FnOnce(F::Item) -> Item2>(self, fc: FC) -> FutureResult<Map<F, FC>> {
    //     match self {
    //         FutureResult::Ok(x) => FutureResult::Ok((fc)(x)),
    //         FutureResult::Err(e) => FutureResult::Err(e),
    //         FutureResult::Wait(f) => FutureResult::Wait(f.map(fc)),
    //     }
    // }

    pub fn to_future(self) -> FutureResultFuture<F> {
        match self {
            FutureResult::Ok(item) => FutureResultFuture::Ok(item),
            FutureResult::Err(e) => FutureResultFuture::Err(e),
            FutureResult::Wait(f) => FutureResultFuture::Wait(f),
        }
    }
}


/// A future version of FutureResult. See `FutureResult::to_future(self)`.
pub enum FutureResultFuture<F: Future> {
    Ok(F::Item),
    Err(F::Error),
    Wait(F),
    /// A consumed FutureResultFuture. This exists so poll can move out of the Ok and Err states.
    /// Polling a consumed FutureResultFuture is an error.
    Consumed,
}

impl<F: Future> Future for FutureResultFuture<F> {
    type Item = F::Item;
    type Error = F::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut oldself = FutureResultFuture::Consumed;
        mem::swap(self, &mut oldself);

        match oldself {
            FutureResultFuture::Ok(item) => Ok(Async::Ready(item)),
            FutureResultFuture::Err(err) => Err(err),
            FutureResultFuture::Wait(mut f) => {
                let r = f.poll();
                // We don't actually need to check what we polled.
                // We just keep the future around; it can be polled again if needed.
                *self = FutureResultFuture::Wait(f);
                r
            }
            FutureResultFuture::Consumed => panic!("Cannot poll a complete future twice"),
        }
    }
}

/*
TODO: where to put Out?
ClosedChain has an Out parameter that is now real inconvenient. WHY is it inconvenient?
Because it's really C::Out. Ugh.

We want FutureChain's generic type to be real pure.

We might have to make FutureChain a wrapper...

TODO: the below doesn't work because of static requirements.
*/
/// A state containing an Item, Error, or ClosedChain. Used as input to FutureChains.
///
/// Ideally, ClosedChain should be paramterized by output and error type.
/// I was unable to make this work without inducing ugly type signatures.
pub enum FutureChainState<Item> {
    Ok(Item),
    Err(io::Error),
    Wait(ClosedChain),
}

// Alternate idea:
// pub struct FutureChain2<C> where C: Chain {

// }

/// A trait for chains that carry FutureChainResults. These are automatically Chains (...)
pub trait FutureChain<Input> where Self: Chain<FutureChainState<Input>, Out = FutureChainState<()>> + 'static {
    /// Execute this FutureChain with the given item.
    fn exec_ok(self, i: Input) -> Self::Out  {
        self.exec(FutureChainState::Ok(i))
    }

    /// Execute this FutureChain with the given error. The default behavior is to return the error immediately.
    fn exec_err(self, e: io::Error) -> Self::Out {
        self.exec(FutureChainState::Err(e))
    }

    /// Delay execution of this FutureChain, instead returning a Wait.
    fn wait<F: Future<Item = Input, Error = io::Error> + 'static>(self, f: F) -> Self::Out {
        FutureChainState::Wait(ClosedChain::new(f, self))
    }
}

impl<Input, C> FutureChain<Input> for C where
C: Chain<FutureChainState<Input>, Out = FutureChainState<()>> + 'static {
}

/// Like bind, but for a fn that is only interested in Ok(item) results.
/// If given an Err or Wait, it is passed to the Chain directly.
fn bind_ok<F, C, Item, L>(link: F, c: C) -> impl FutureChain<Item> where
F: FnOnce(Item, C) -> C::Out + 'static,
C: FutureChain<L>,
Item: 'static,
L: 'static,
{
    let mylink = |fcr, c2| match fcr {
        FutureChainState::Ok(i) => (link)(i, c2),
        FutureChainState::Err(e) => c2.exec_err(e),
        FutureChainState::Wait(s) => c2.exec(FutureChainState::Wait(s)),
    };

    bind(mylink, c)
}

/// Like bind, but for a fn that is only interested in Err(e) results.
/// If given an Ok or Wait, it is passed to the Chain directly.
fn bind_catch<F, C, Item>(catch: F, c: C) -> impl FutureChain<Item> where
F: FnOnce(io::Error, C) -> C::Out + 'static,
C: FutureChain<Item>,
Item: 'static,
{
    let mycatch = |fcr, c2: C| {
        let r: FutureChainState<()> = match fcr {
            FutureChainState::Ok(i) => {
                let x: FutureChainState<()> = c2.exec_ok(i);
                x
            },
            FutureChainState::Err(e) => (catch)(e, c2),
            FutureChainState::Wait(s) => c2.exec(FutureChainState::Wait(s)),

        };
        r
    };

    bind(mycatch, c)
}

trait ClosedChainInner {
    fn poll(&mut self) -> FutureResult<ClosedChain>;
}

// TODO eliminate io::Error
struct ClosedChainInnerImpl<F, C> where
F: Future<Error = io::Error>,
C: FutureChain<F::Item>,
{
    b: Option<(F, C)>,
}

impl<F, C> ClosedChainInnerImpl<F, C> where
F: Future<Error = io::Error>,
C: FutureChain<F::Item>,
{
    fn new(f: F, chain: C) -> Self {
        ClosedChainInnerImpl {
            b: Some((f, chain)),
        }
    }
}

impl<F, C> ClosedChainInner for ClosedChainInnerImpl<F, C> where
F: Future<Error = io::Error> + 'static,
C: FutureChain<F::Item>,
{
    fn poll(&mut self) -> FutureResult<ClosedChain> {
        let unbox = match mem::replace(&mut self.b, None) {
            Some(tuple) => tuple,
            None => panic!("cannot poll a used Future"),
        };

        let (mut f, link) = unbox;

        let x = match f.poll() {
            Ok(Async::Ready(i)) => link.exec(FutureChainState::Ok(i)),
            Ok(Async::NotReady) => FutureChainState::Wait(ClosedChain::wrap(Self::new(f, link))),
            Err(e) => link.exec(FutureChainState::Err(e)),
        };

        // TODO ???
        match x {
            FutureChainState::Ok(()) => FutureResult::Ok(()),
            FutureChainState::Err(e) => FutureResult::Err(e),
            FutureChainState::Wait(cc) => FutureResult::Wait(cc),
        }
    }
}

pub struct ClosedChain {
    inner: Box<ClosedChainInner>,
}

impl ClosedChain {
    fn new<F, C>(f: F, chain: C) -> Self where
    F: Future<Error = io::Error> + 'static,
    C: FutureChain<F::Item> + 'static,
    {
        Self::wrap(ClosedChainInnerImpl::new(f, chain))
    }

    fn wrap<F, C>(cc: ClosedChainInnerImpl<F, C>) -> Self where
    F: Future<Error = io::Error> + 'static,
    C: FutureChain<F::Item> + 'static,
    {
        ClosedChain {
            inner: Box::new(cc),
        }
    }
}

impl Future for ClosedChain {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(), io::Error> {
        match self.inner.poll() {
            FutureResult::Ok(i) => Ok(Async::Ready(i)),
            FutureResult::Err(e) => Err(e),
            FutureResult::Wait(mut cc) => {
                mem::swap(self, &mut cc);
                Ok(Async::NotReady)
            }
        }
    }
}

// pub struct ClosedChain {
//     poll_f: fn(*mut u8),
//     drop_f: fn(*mut u8),
//     target: *mut u8,
// }

// impl ClosedChain {
//     fn new<F: Future, C: FutureChain<F::Item, F::Error>>(f: F, chain: C) -> Self {
//         Self::wrap(ClosedChainInner::new(f, chain))
//     }

//     /// Internal poll function. This consumes the target pointer; the caller must set it to null before calling.
//     fn poll_f<F, C>(target: *mut u8) where
//     F: Future,
//     C: FutureChain<F::Item, F::Error>,
//     {
//         unsafe {
//             let typed_target = mem::transmute(target);
//             let target_box: Box<ClosedChainInner<F, C>> = Box::from_raw(typed_target);
//             target_box.poll()
//         }
//     }

//     /// Internal drop function, called by <Self as Drop>::drop.
//     fn drop_f<F, C>(target: *mut u8) where
//     F: Future,
//     C: FutureChain<F::Item, F::Error>,
//     {
//          unsafe {
//             let typed_target = mem::transmute(target);
//             let _b: Box<ClosedChainInner<F, C>> = Box::from_raw(typed_target);
//             // _b is now dropped
//         }
//     }

//     fn wrap<F, C>(cc: ClosedChainInner<F, C>) -> Self where
//     F: Future,
//     C: FutureChain<F::Item, F::Error>,
//     {
//         ClosedChain {
//             poll_f: Self::poll_f::<F, C>,
//             drop_f: Self::drop_f::<F, C>,
//             target: Box::into_raw(Box::new(cc)) as *mut u8,
//         }
//     }

//     fn poll(mut self) {
//         let target = self.target;
//         self.target = ptr::null::<u8>() as *mut _;
//         (self.poll_f)(target)
//     }
// }

// impl Drop for ClosedChain {
//     fn drop(&mut self) {
//         if self.target as *const _ != ptr::null() {
//             // Put the null check in the static fn, so it's not hidden behind an indirection.
//             // Then it will be optimized away most of the time.
//             (self.drop_f)(self.target);
//         }
//         // Not needed, but for completeness' sake
//         mem::drop(self.poll_f);
//         mem::drop(self.drop_f);
//     }
// }
