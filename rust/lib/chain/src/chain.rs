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

use std::marker::PhantomData;
use std::mem;
use std::ptr;

/// A Chain is a computation that takes in an input and yields an output,
/// where the input may be a Future. Chains may be linked together, hence the name.
/// Chains do nothing until converted into a ChainState and executed as a Future.
pub trait Chain<Input>: Sized {
    type Output;

    // /// Links the given function to this Chain, returning an new Chain.
    // /// Link is a function that accepts an input item A and this Chain and produces a ChainState.
    // /// Link may be an arbitrary function--it may call Chain::close, construct a new Chain and call chain::close
    // /// on that, or even return, ignoring the rest of this chain.
    // fn prepend<C>(self, c: C) -> ChainPair<C, Self> where
    // C: Chain<Output = Self::Input>;

    fn prepend<F, I>(self, f: F) -> ChainLink<F, Self, I, Input> where
    F: FnOnce(I, Self) -> Self::Output {
        ChainLink::new(f, self)
    }

    // /// Links two chains together, feeding the output of the given chain into this one.
    // fn prepend_chain<C, I>(self, c: C) -> ChainPair<C, Self, I, Input> where
    // C: Chain<I, Output = Input> {
    //     ChainPair::new(c, self)
    // }

    fn indir(self) -> Indir<Input, Self::Output> where Self: 'static {
        Indir::new(self)
    }

    fn exec(self, i: Input) -> Self::Output;
}

// TODO rename
pub struct EmptyChain;

impl EmptyChain {
    pub fn new() -> Self {
        EmptyChain
    }
}

impl<T> Chain<T> for EmptyChain {
    type Output = T;

    fn exec(self, i: T) -> Self::Output {
        i
    }
}

// TODO: name
// TODO: impl is slow
pub struct Indir<I, O> {
    // type is Box<C> where C is a 'hidden' type
    inner: *mut u8,
    exec_f: fn(*mut u8, I) -> O,
    drop_box_f: fn(*mut u8),
}

impl<I, O> Indir<I, O> {
    fn new<C: Chain<I, Output = O> + 'static>(c: C) -> Self {
        Indir {
            inner: Box::into_raw(Box::new(c)) as *mut _,
            exec_f: Self::exec_f::<C>,
            drop_box_f: Self::drop_box_f::<C>,
        }
    }

    /// Internal exec function. This consumes the target pointer; the caller must set it to null before calling.
    fn exec_f<C: Chain<I, Output = O>>(target: *mut u8, i: I) -> O {
        unsafe {
            let typed_target = mem::transmute(target);
            let target_box: Box<C> = Box::from_raw(typed_target);
            target_box.exec(i)
        }
    }

    /// Internal drop function, called by <Self as Drop>::drop.
    fn drop_box_f<C: Chain<I, Output = O>>(target: *mut u8) {
        if target as *const _ != ptr::null() {
            unsafe {
                let typed_target = mem::transmute(target);
                let _b: Box<C> = Box::from_raw(typed_target);
                // _b is now dropped
            }
        }
    }
}

impl<I, O> Chain<I> for Indir<I, O> {
    type Output = O;

    fn exec(mut self, i: I) -> O {
        let r = (self.exec_f)(self.inner, i);
        self.inner = ptr::null::<u8>() as *mut _;
        r
    }

    fn indir(self) -> Indir<I, O> {
        self
    }
}

impl<I, O> Drop for Indir<I, O> {
    fn drop(&mut self) {
        (self.drop_box_f)(self.inner);
        mem::drop(self.exec_f);
        mem::drop(self.drop_box_f);
    }
}

/// An instance of PhantomData that implements Send.
pub struct PhantomDataSend<T>(PhantomData<T>);

unsafe impl<T> Send for PhantomDataSend<T> {}

pub struct ChainLink<F, C, I, _X> where
F: FnOnce(I, C) -> C::Output,
C: Chain<_X>,
{
    f: F,
    c: C,
    // We never use I/L
    _p: PhantomDataSend<*const (I, _X)>,
}

impl<F, C, I, _X> ChainLink<F, C, I, _X> where
F: FnOnce(I, C) -> C::Output,
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
F: FnOnce(I, C) -> C::Output,
C: Chain<_X>,
{
    type Output = C::Output;

    fn exec(self, i: I) -> C::Output {
        (self.f)(i, self.c)
    }
}

// pub struct ChainPair<A, B, I, L> where
// A: Chain<I, Output = L>,
// B: Chain<L>,
// {
//     a: A,
//     b: B,
//     // We never store an I.
//     _p: PhantomDataSend<*const I>,
// }

// impl<A, B, I, L> ChainPair<A, B, I, L> where
// A: Chain<I, Output = L>,
// B: Chain<L>,
// {
//     fn new(a: A, b: B) -> Self {
//         ChainPair {
//             a: a,
//             b: b,
//             _p: PhantomDataSend(PhantomData),
//         }
//     }
// }

// TODO: different module. different binary? need to test inlining
#[cfg(test)]
mod tests {
    use super::*;

    use test::{Bencher, black_box};

    #[bench]
    fn bench_factorial_rust(b: &mut Bencher) {
        b.iter(|| {
            fn fact(i: u64) -> u64 {
                black_box(i);

                if i == 1 {
                    1
                } else {
                    i * fact(i - 1)
                }
            }

            let x = fact(20);
            black_box(x);
            debug_assert!(x == 2432902008176640000);
        })
    }

    #[bench]
    fn bench_factorial_chain(b: &mut Bencher) {
        fn fact<C: Chain<u64> + 'static>(i: u64, rest: C) -> C::Output {
            black_box(i);

            if i == 1 {
                rest.exec(i)
            } else {
                fact(i - 1, rest.prepend(move |j, rest| rest.exec(j * i)).indir())
            }
        }

        b.iter(|| {
            let c = EmptyChain::new();
            let x = fact(20, c);
            black_box(x);
            debug_assert!(x == 2432902008176640000);
        })
    }

    // Should take 0 ns after optimization.
    #[bench]
    fn bench_shallow_inlining_rust(b: &mut Bencher) {
        b.iter(|| {
            let a = || 1 as u64;
            let b = || a() + a();
            let c = || b() + b();
            let d = || c() + c();

            let x = d();
            black_box(x);
            debug_assert!(x == 8);
        })
    }

    // Ideally this should take 0 ns, like the above.
    #[bench]
    #[allow(dead_code)]
    fn bench_shallow_inlining_chain(b: &mut Bencher) {
        b.iter(|| {
            let chain = EmptyChain::new();

            fn a<C: Chain<u64>>(ch: C) -> C::Output {
                ch.exec(1)
            }

            fn b<C: Chain<u64>>(ch: C) -> C::Output {
                a(ch.prepend(|x, rest| a(rest.prepend(|y, rest| rest.exec(x + y)))))
            }

            fn c<C: Chain<u64>>(ch: C) -> C::Output {
                b(ch.prepend(|x, rest| b(rest.prepend(|y, rest| rest.exec(x + y)))))
            }

            fn d<C: Chain<u64>>(ch: C) -> C::Output {
                c(ch.prepend(|x, rest| c(rest.prepend(|y, rest| rest.exec(x + y)))))
            }

            let x = d(chain);
            black_box(x);
            debug_assert!(x == 8);
        })
    }
}
