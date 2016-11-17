//! TODO docs
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

use std::mem;

use futures::{Async, Future, Poll};

use chain::{bind, Chain};

/// The result of a fn that may optionally return a future.
pub enum FutureResult<F: Future> {
    Ok(F::Item),
    Err(F::Error),
    Wait(F),
}

/// A state containing an Item, Error, or ClosedChain. Used as input to FutureChains.
///
/// Ideally, ClosedChain should be paramterized by output and error type.
/// I was unable to make this work without inducing ugly type signatures.
/// So it's parameterized by (), (), and you have to side-effect returns and errors.
pub enum ChainState<I, E> {
    Ok(I),
    Err(E),
    Wait(ClosedChain),
}

/// A trait for chains that carry FutureChainResults. These automatically implement Chain<ChainState<I, E>>.
pub trait FutureChain<I, E> where Self: Chain<ChainState<I, E>, Out = Option<ClosedChain>> + 'static {
    /// Execute this FutureChain with the given item.
    fn exec_ok(self, i: I) -> Self::Out  {
        self.exec(ChainState::Ok(i))
    }

    /// Execute this FutureChain with the given error. The default behavior is to return the error immediately.
    fn exec_err(self, e: E) -> Self::Out {
        self.exec(ChainState::Err(e))
    }

    /// Delay execution of this FutureChain, instead returning a Wait.
    fn wait<F: Future<Item = I, Error = E> + 'static>(self, f: F) -> Self::Out {
        Some(ClosedChain::new(f, self))
    }
}

impl<I, E, C> FutureChain<I, E> for C where
C: Chain<ChainState<I, E>, Out = Option<ClosedChain>> + 'static {
}

/// Like bind, but for a fn that is only interested in Ok(item) results.
/// If given an Err or Wait, it is passed to the Chain directly.
pub fn bind_ok<F, C, I, E, L>(link: F, c: C) -> impl FutureChain<I, E> where
F: FnOnce(I, C) -> C::Out + 'static,
C: FutureChain<L, E>,
I: 'static,
E: 'static,
L: 'static,
{
    let mylink = |fcr, c2| match fcr {
        ChainState::Ok(i) => (link)(i, c2),
        ChainState::Err(e) => c2.exec_err(e),
        ChainState::Wait(s) => c2.exec(ChainState::Wait(s)),
    };

    bind(mylink, c)
}

pub fn premap_ok<F, C, I, E, O>(f: F, c: C) -> impl FutureChain<I, E> where
F: FnOnce(I) -> O + 'static,
C: FutureChain<O, E>,
I: 'static,
E: 'static,
O: 'static,
{
    bind_ok(|i, c| c.exec_ok(f(i)), c)
}

/// Like bind, but for a fn that is only interested in Err(e) results.
/// If given an Ok or Wait, it is passed to the Chain directly.
pub fn bind_catch<F, C, I, E, L>(catch: F, c: C) -> impl FutureChain<I, E> where
F: FnOnce(E, C) -> C::Out + 'static,
C: FutureChain<I, L>,
I: 'static,
E: 'static,
L: 'static,
{
    let mycatch = |fcr, c2: C| match fcr {
        ChainState::Ok(i) => c2.exec_ok(i),
        ChainState::Err(e) => (catch)(e, c2),
        ChainState::Wait(s) => c2.exec(ChainState::Wait(s)),
    };

    bind(mycatch, c)
}

trait ClosedChainInner {
    fn poll(&mut self) -> Option<ClosedChain>;
}

struct ClosedChainInnerImpl<F, C> where
F: Future,
C: FutureChain<F::Item, F::Error>,
{
    b: Option<(F, C)>,
}

impl<F, C> ClosedChainInnerImpl<F, C> where
F: Future,
C: FutureChain<F::Item, F::Error>,
{
    fn new(f: F, chain: C) -> Self {
        ClosedChainInnerImpl {
            b: Some((f, chain)),
        }
    }
}

impl<F, C> ClosedChainInner for ClosedChainInnerImpl<F, C> where
F: Future + 'static,
C: FutureChain<F::Item, F::Error>,
{
    fn poll(&mut self) -> Option<ClosedChain> {
        let unbox = match mem::replace(&mut self.b, None) {
            Some(tuple) => tuple,
            None => panic!("cannot poll a used Future"),
        };

        let (mut f, link) = unbox;

        // TODO: This can be slightly optimized by using a single indirection,
        // at the expense of having to store a function pointer for the drop function.
        match f.poll() {
            Ok(Async::Ready(i)) => link.exec_ok(i),
            Ok(Async::NotReady) => Some(ClosedChain::wrap(Self::new(f, link))),
            Err(e) => link.exec_err(e),
        }
    }
}

pub struct ClosedChain {
    inner: Box<ClosedChainInner>,
}

impl ClosedChain {
    fn new<F, C>(f: F, chain: C) -> Self where
    F: Future + 'static,
    C: FutureChain<F::Item, F::Error> + 'static,
    {
        Self::wrap(ClosedChainInnerImpl::new(f, chain))
    }

    fn wrap<F, C>(cc: ClosedChainInnerImpl<F, C>) -> Self where
    F: Future + 'static,
    C: FutureChain<F::Item, F::Error> + 'static,
    {
        ClosedChain {
            inner: Box::new(cc),
        }
    }
}

impl Future for ClosedChain {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<(), ()> {
        match self.inner.poll() {
            Some(mut cc) => {
                mem::swap(self, &mut cc);
                Ok(Async::NotReady)
            },
            None => Ok(Async::Ready(())),
        }
    }
}

// TODO: below might be faster... Who knows.

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
