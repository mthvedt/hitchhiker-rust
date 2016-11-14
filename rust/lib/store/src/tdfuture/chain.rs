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
//! Continuation-passing code is way too complex for our use case, however, so this case is not implemented.

use std::io;
use std::marker::PhantomData;
use std::mem;
use std::ptr;

use futures::{Async, Future, Map, Poll};

use alloc::{ScopedValue};
use tdfuture::FutureResult;

/// An uninstantiable Future. Useful for type gymnastics.
struct VoidFuture<Item, Error> {
    phantom: PhantomData<(Item, Error)>,
}

impl<Item, Error> Future for VoidFuture<Item, Error> {
    type Item = Item;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        unreachable!()
    }
}

/// A Chain is a computation that takes in an input and yields an output,
/// where the input may be a Future. Chains may be linked together, hence the name.
/// Chains do nothing until converted into a ChainState and executed as a Future.
pub trait Chain<Input>: Sized {
    type Output;
    type Context: ChainContext;

    // /// Links the given function to this Chain, returning an new Chain.
    // /// Link is a function that accepts an input item A and this Chain and produces a ChainState.
    // /// Link may be an arbitrary function--it may call Chain::close, construct a new Chain and call chain::close
    // /// on that, or even return, ignoring the rest of this chain.
    // fn prepend<C>(self, c: C) -> ChainPair<C, Self> where
    // C: Chain<Output = Self::Input>;

    fn prepend<F, I>(self, f: F) -> ChainLink<F, Self, I, Input> where
    F: FnOnce(I, Self) -> ChainState<Self::Output> {
        ChainLink::new(f, self)
    }

    /// Links two chains together, feeding the output of the given chain into this one.
    fn prepend_chain<C, I>(self, c: C) -> ChainPair<C, Self, I, Input> where
    C: Chain<I, Output = Input> {
        ChainPair::new(c, self)
    }

    /// Links two chains together, feeding the output of this chain into the given one.
    // TODO: should this be allowed?
    fn append_chain<C>(self, c: C) -> ChainPair<Self, C, Input, Self::Output> where
    C: Chain<Self::Output> {
        ChainPair::new(self, c)
    }

    fn indir(self) -> Indir<Input, Self::Output, Self::Context> where Self: 'static {
        Indir::new(self)
    }

    /// Plugs a FutureResult into this Chain, returning a ChainState which may be executed.
    // TODO: this does not need to be part of Chain. Chain is a more general continuation-passing mechanism.
    fn recv<F: Future<Item = Input, Error = io::Error>>(self, input: FutureResult<F>) ->
    ChainState<Self::Output> {
        match input {
            FutureResult::Ok(i) => self.exec(i),
            FutureResult::Err(e) => ChainState::Err(e),
            FutureResult::Wait(f) => ChainState::Wait(ChainExecutor::wrap(ClosedChain::new(f, self)))
        }
    }

    fn recv_ok(self, item: Input) -> ChainState<Self::Output> {
        self.recv::<VoidFuture<Input, io::Error>>(FutureResult::Ok(item))
    }

    fn recv_err(self, err: io::Error) -> ChainState<Self::Output> {
        self.recv::<VoidFuture<Input, io::Error>>(FutureResult::Err(err))
    }

    fn exec(self, i: Input) -> ChainState<Self::Output>;

    /// Not for external use.
    fn context(&mut self) -> &mut Self::Context;
}

pub trait ChainContext: Clone {
    // /// This is not yet used.
    // fn alloc<F: Future>(&mut self, f: F) -> ScopedValue<F>;
}

#[derive(Clone)]
struct ChainContextImpl;

impl ChainContext for ChainContextImpl {}

pub struct EmptyChain {
    c: ChainContextImpl,
}

impl EmptyChain {
    fn new() -> Self {
        EmptyChain {
            c: ChainContextImpl,
        }
    }
}

impl<T> Chain<T> for EmptyChain {
    type Output = T;
    type Context = ChainContextImpl;

    fn exec(self, i: T) -> ChainState<Self::Output> {
        ChainState::Ok(i)
    }

    fn context(&mut self) -> &mut Self::Context {
        &mut self.c
    }
}

// TODO: name
// TODO: impl is slow
pub struct Indir<I, O, CC: ChainContext> {
    // type is Box<C> where C is a 'hidden' type
    inner: *mut u8,
    context_f: fn(*mut u8) -> *mut CC,
    exec_f: fn(*mut u8, I) -> ChainState<O>,
    drop_f: fn(*mut u8),
    _p: PhantomData<*const CC>,
}

impl<I, O, CC: ChainContext> Indir<I, O, CC> {
    fn new<C: Chain<I, Output = O, Context = CC> + 'static>(c: C) -> Self {
        Indir {
            inner: Box::into_raw(Box::new(c)) as *mut _,
            context_f: Self::context_f::<C>,
            exec_f: Self::exec_f::<C>,
            drop_f: Self::drop_f::<C>,
            _p: PhantomData,
        }
    }

    fn context_f<C: Chain<I, Output = O, Context = CC>>(target: *mut u8) -> *mut CC {
        unsafe {
            let typed_target = mem::transmute(target);
            let mut target_box: Box<C> = Box::from_raw(typed_target);
            let r = target_box.context() as *mut _;
            Box::into_raw(target_box); // don't auto-drop the pointer
            r
        }
    }

    /// Internal exec function. This consumes the target pointer; the caller must set it to null before calling.
    fn exec_f<C: Chain<I, Output = O, Context = CC>>(target: *mut u8, i: I) -> ChainState<O> {
        unsafe {
            let typed_target = mem::transmute(target);
            let target_box: Box<C> = Box::from_raw(typed_target);
            target_box.exec(i)
        }
    }

    /// Internal drop function, called by <Self as Drop>::drop.
    fn drop_f<C: Chain<I, Output = O, Context = CC>>(target: *mut u8) {
        if target as *const _ != ptr::null() {
            unsafe {
                let typed_target = mem::transmute(target);
                let target_box: Box<CC> = Box::from_raw(typed_target);
                // target_box is now dropped
            }
        }
    }
}

impl<I, O, CC: ChainContext> Chain<I> for Indir<I, O, CC> {
    type Output = O;
    type Context = CC;

    fn exec(self, i: I) -> ChainState<Self::Output> {
        (self.exec_f)(self.inner, i)
    }

    /// Not for external use.
    fn context(&mut self) -> &mut Self::Context {
        unsafe {
            &mut *(self.context_f)(self.inner)
        }
    }

    fn indir(self) -> Indir<I, O, CC> {
        self
    }
}

impl<I, O, CC: ChainContext> Drop for Indir<I, O, CC> {
    fn drop(&mut self) {
        (self.drop_f)(self.inner);
        mem::drop(self.context_f);
        mem::drop(self.exec_f);
        mem::drop(self.drop_f);
        mem::drop(self._p);
    }
}

/// An instance of PhantomData that implements Send.
pub struct PhantomDataSend<T>(PhantomData<T>);

unsafe impl<T> Send for PhantomDataSend<T> {}

pub struct ChainLink<F, C, I, _X> where
F: FnOnce(I, C) -> ChainState<C::Output>,
C: Chain<_X>,
{
    f: F,
    c: C,
    // We never use I/L
    _p: PhantomDataSend<*const (I, _X)>,
}

impl<F, C, I, _X> ChainLink<F, C, I, _X> where
F: FnOnce(I, C) -> ChainState<C::Output>,
C: Chain<_X>
{
    fn new(f: F, c: C) -> Self {
        ChainLink {
            f: f,
            c: c,
            _p: PhantomDataSend(PhantomData),
        }
    }
}

impl<F, C, I, _X> Chain<I> for ChainLink<F, C, I, _X> where
F: FnOnce(I, C) -> ChainState<C::Output>,
C: Chain<_X>,
{
    type Output = C::Output;
    type Context = C::Context;

    fn exec(self, i: I) -> ChainState<C::Output> {
        (self.f)(i, self.c)
    }

    fn context(&mut self) -> &mut Self::Context {
        self.c.context()
    }
}

pub struct ChainPair<A, B, I, L> where
A: Chain<I, Output = L>,
B: Chain<L>,
{
    a: A,
    b: B,
    // We never store an I.
    _p: PhantomDataSend<*const I>,
}

impl<A, B, I, L> ChainPair<A, B, I, L> where
A: Chain<I, Output = L>,
B: Chain<L>,
{
    fn new(a: A, b: B) -> Self {
        ChainPair {
            a: a,
            b: b,
            _p: PhantomDataSend(PhantomData),
        }
    }
}

/*
A plan to make this faster:
When a chain is 'closed', it is moved onto the heap. A sort of stack is created (living on the heap,
separate from the main stack). As links in the chain are consumed, the stack pointer moves forward,
but the chain links can push it backwards and add more state.

Essentially, a sort of stack, separate from the main stack, is created.

However, chains will generally be small and copying their internals generally OK speed-wise--not super fast,
but faster than malloc, and on the fast path (future returns immediately) there is no copy.
So let's see if this becomes necessary perf-wise.
*/
struct ClosedChain<F, C, I> where
F: Future<Item = I, Error = io::Error>,
C: Chain<F::Item>,
{
    f: F,
    link: C,
}

impl<F, C, I> ClosedChain<F, C, I> where
F: Future<Item = I, Error = io::Error>,
C: Chain<F::Item>,
{
    fn new(f: F, chain: C) -> Self {
        ClosedChain {
            f: f,
            link: chain,
        }
    }

    fn poll(mut self) -> ChainState<C::Output> {
        match self.f.poll() {
            Ok(Async::Ready(i)) => self.link.exec(i),
            Ok(Async::NotReady) => ChainState::Wait(ChainExecutor::wrap(self)),
            Err(e) => ChainState::Err(e),
        }
    }
}

struct ChainExecutor<T> {
    poll_f: fn(*mut u8) -> ChainState<T>,
    drop_f: fn(*mut u8),
    target: *mut u8,
    // We never store a T.
    // TODO: impl Send.
    _p: PhantomData<*const T>,
}

impl<T> ChainExecutor<T> {
    /// Internal poll function. This consumes the target pointer; the caller must set it to null before calling.
    fn poll_f<F, C, I>(target: *mut u8) -> ChainState<T> where
    F: Future<Item = I, Error = io::Error>,
    C: Chain<F::Item, Output = T>,
    {
        unsafe {
            let typed_target = mem::transmute(target);
            let target_box: Box<ClosedChain<F, C, I>> = Box::from_raw(typed_target);
            target_box.poll()
        }
    }

    /// Internal drop function, called by <Self as Drop>::drop.
    fn drop_f<F, C, I>(target: *mut u8) where
    F: Future<Item = I, Error = io::Error>,
    C: Chain<F::Item, Output = T>,
    {
        if target as *const _ != ptr::null() {
            unsafe {
                let typed_target = mem::transmute(target);
                let target_box: Box<ClosedChain<F, C, I>> = Box::from_raw(typed_target);
                // target_box is now dropped
            }
        }
    }

    fn wrap<F, C, I>(cc: ClosedChain<F, C, I>) -> Self where
    F: Future<Item = I, Error = io::Error>,
    C: Chain<F::Item, Output = T>,
    {
        unsafe {
            ChainExecutor {
                poll_f: Self::poll_f::<F, C, I>,
                drop_f: Self::drop_f::<F, C, I>,
                target: Box::into_raw(Box::new(cc)) as *mut u8,
                _p: PhantomData,
            }
        }
    }

    fn poll(mut self) -> ChainState<T> {
        let target = self.target;
        // In case poll_f panics
        self.target = ptr::null::<u8>() as *mut _;
        (self.poll_f)(target)
    }
}

impl<T> Drop for ChainExecutor<T> {
    fn drop(&mut self) {
        (self.drop_f)(self.target);
        // Not needed, but for completeness' sake
        mem::drop(self.poll_f);
        mem::drop(self.drop_f);
    }
}

/// A Chain that is partially or completely executed. If partially executed, it is waiting for
/// a Future to return.
///
/// ChainStates are produced from Chain::recv.
pub enum ChainState<T>
{
    Wait(ChainExecutor<T>),
    Ok(T),
    Err(io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    use test::{Bencher, black_box};

    fn fact<C: Chain<u64> + 'static>(i: u64, rest: C) -> ChainState<C::Output> {
        black_box(i);

        if i == 1 {
            rest.recv_ok(i)
        } else {
            fact(i - 1, rest.prepend(move |j, rest| rest.recv_ok(j * i)).indir())
        }
    }

    #[test]
    fn test_factorial() {
        let c = EmptyChain::new();
        let o = fact(5, c);
        match o {
            ChainState::Wait(_) => panic!(),
            ChainState::Ok(t) => assert!(t == 120),
            ChainState::Err(e) => panic!(),
        }
    }

    #[bench]
    fn bench_normal_factorial(b: &mut Bencher) {
        b.iter(|| {
            fn fact(i: u64) -> u64 {
                black_box(i);

                if i == 1 {
                    1
                } else {
                    i * fact(i - 1)
                }
            }

            black_box(fact(20));
        })
    }

    #[bench]
    fn bench_rust_factorial(b: &mut Bencher) {
        b.iter(|| {
            let c = EmptyChain::new();
            let o = fact(20, c);

            match o {
                ChainState::Wait(_) => panic!(),
                ChainState::Ok(t) => black_box(t),
                ChainState::Err(e) => panic!(),
            };
        })
    }
}
