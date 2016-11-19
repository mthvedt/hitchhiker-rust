use std::marker::PhantomData;
use std::mem;
use std::ptr;

/*
A plan to make this faster:
When a chain is 'closed', it is moved onto the heap. A sort of stack is created (living on the heap,
separate from the main stack). As links in the chain are consumed, the stack pointer moves forward,
but the chain links can push it backwards and add more state.

Essentially, a sort of stack, separate from the main stack, is created.

However, chains will generally be small and copying their internals generally OK speed-wise--not super fast,
but faster than malloc, and on the fast path (future returns immediately) there is no copy.
So let's see if this becomes necessary perf-wise.

We have laid the groundwork for this by making prepend a trait method.
*/

/// A Chain is a computation that takes in an input and yields an output.
/// Chains may have functions 'linked' to the beginning, hence the name.
///
/// Note that two chains cannot be linked together. This is due to Rust's lack of higher-kinded types.
/// Chains must always be passed around in tail position. This is not necessarily a disadvantage,
/// since by doing so, you sidestep Rust's poor support for abstract return types.
/// Indeed, one of the motivation factors of Chain was to avoid abstract return types.
pub trait Chain<Input>: Sized {
    // TODO: Out -> Output.
    type Out;

    /// Returns an indirected Chain from this one, discarding all internal type information.
    /// Each indirect must chase a pointer once when it executes.
    ///
    /// Indirects have two uses:
    /// * They can make code compile when the static types would otherwise be infinite,
    /// for instance, when functions are recursive.
    /// * They can be used to box a Chain of unknown size into an allocated Chain of fixed size.
    fn indir<'a>(self) -> Indir<'a, Input, Self::Out> where Self: 'a {
        Indir::new(self)
    }

    /// Execute this Chain, yielding output.
    fn exec(self, i: Input) -> Self::Out;
}

/// Prepends the given function to the given Chain.
/// The link function is a function that accepts an input item A and this Chain, and produces this chain's Output.
/// In idiomatic use, type inference is used to deduce the type of F.
pub fn bind<F, C, I, L>(f: F, c: C) -> impl Chain<I, Out = C::Out> where
C: Chain<L>,
F: FnOnce(I, C) -> C::Out
{
    ChainLink::new(f, c)
}

pub fn premap<F, C, I, L>(f: F, c: C) -> impl Chain<I, Out = C::Out> where
C: Chain<L>,
F: FnOnce(I) -> L
{
    bind(|i, c| c.exec(f(i)), c)
}

/// An empty chain that returns the identity when it executes.
pub struct EmptyChain;

impl EmptyChain {
    pub fn new() -> Self {
        EmptyChain
    }
}

impl<T> Chain<T> for EmptyChain {
    type Out = T;

    fn exec(self, i: T) -> T {
        i
    }
}

/// Executes the given function with the empty chain.
pub fn exec<F, O>(f: F) -> O where F: FnOnce(EmptyChain) -> O {
    f(EmptyChain::new())
}

// TODO: name
// TODO: impl is slow
pub struct Indir<'a, I, O> {
    // type is Box<C> where C is a 'hidden' type implementing Chain<I, Out = O>
    inner: *mut u8,
    exec_f: fn(*mut u8, I) -> O,
    drop_box_f: fn(*mut u8),
    // Important for drop checking, I think. (Rust docs don't really explain.)
    _p: PhantomData<&'a u8>,
}

impl<'a, I, O> Indir<'a, I, O> {
    fn new<C: Chain<I, Out = O> + 'a>(c: C) -> Self {
        Indir {
            inner: Box::into_raw(Box::new(c)) as *mut _,
            exec_f: Self::exec_f::<C>,
            drop_box_f: Self::drop_box_f::<C>,
            _p: PhantomData,
        }
    }

    /// Internal exec function. This consumes the target pointer; the caller must set it to null before calling.
    fn exec_f<C: Chain<I, Out = O>>(target: *mut u8, i: I) -> O {
        debug_assert!(target as *const _ != ptr::null());
        unsafe {
            let typed_target = mem::transmute(target);
            let target_box: Box<C> = Box::from_raw(typed_target);
            target_box.exec(i)
        }
    }

    /// Internal drop function, called by <Self as Drop>::drop.
    fn drop_box_f<C: Chain<I, Out = O>>(target: *mut u8) {
        debug_assert!(target as *const _ != ptr::null());
        unsafe {
            let typed_target = mem::transmute(target);
            let _b: Box<C> = Box::from_raw(typed_target);
            // _b is now dropped
        }
    }
}

impl<'a, I, O> Chain<I> for Indir<'a, I, O> {
    type Out = O;

    fn exec(mut self, i: I) -> O {
        let a = (self.exec_f)(self.inner, i);
        // For performance, important to do these manipulations outside the function pointers.
        // This way the pointer check in Self::drop can possibly be inlined away.
        self.inner = ptr::null::<u8>() as *mut _;
        a
    }

    fn indir<'b>(self) -> Indir<'b, I, O> where Self: 'b {
        // This is safe because everything in Self must outlive 'b.
        // We manually copy instead of implementing Clone.
        Indir {
            inner: self.inner,
            exec_f: self.exec_f,
            drop_box_f: self.drop_box_f,
            _p: PhantomData,
        }
    }
}

impl<'a, I, O> Drop for Indir<'a, I, O> {
    fn drop(&mut self) {
        if self.inner as *const _ != ptr::null() {
            (self.drop_box_f)(self.inner);
        }
        mem::drop(self.exec_f);
        mem::drop(self.drop_box_f);
    }
}

pub struct ChainLink<F, C, I, _X> where
F: FnOnce(I, C) -> C::Out,
C: Chain<_X>,
{
    f: F,
    c: C,
    // We never use I/L
    _p: PhantomData<*const (I, _X)>,
}

impl<F, C, I, _X> ChainLink<F, C, I, _X> where
F: FnOnce(I, C) -> C::Out,
C: Chain<_X>
{
    fn new(f: F, c: C) -> Self {
        ChainLink {
            f: f,
            c: c,
            _p: PhantomData
        }
    }
}

impl<F, C, I, _X> Chain<I> for ChainLink<F, C, I, _X> where
F: FnOnce(I, C) -> C::Out,
C: Chain<_X>,
{
    type Out = C::Out;

    fn exec(self, i: I) -> C::Out {
        (self.f)(i, self.c)
    }
}

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
        fn fact<C: Chain<u64>>(i: u64, rest: C) -> C::Out {
            black_box(i);

            if i == 1 {
                rest.exec(i)
            } else {
                fact(i - 1, bind(move |j, c| c.exec(j * i), rest).indir())
            }
        }

        b.iter(|| {
            let x = exec(|c| fact(20, c));
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
            fn a<C: Chain<u64>>(ch: C) -> C::Out {
                ch.exec(1)
            }

            fn b<C: Chain<u64>>(ch: C) -> C::Out {
                a(bind(|x, ch| a(bind(|y, ch| ch.exec(x + y), ch)), ch))
            }

            fn c<C: Chain<u64>>(ch: C) -> C::Out {
                b(bind(|x, ch| b(bind(|y, ch| ch.exec(x + y), ch)), ch))
            }

            fn d<C: Chain<u64>>(ch: C) -> C::Out {
                c(bind(|x, ch| c(bind(|y, ch| ch.exec(x + y), ch)), ch))
            }

            let x = exec(d);
            black_box(x);
            debug_assert!(x == 8);
        })
    }
}
