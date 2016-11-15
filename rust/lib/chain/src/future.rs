use std::mem;

use futures::{Async, Future, Poll};

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

pub struct ChainExecutor<T> {
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
                let _b: Box<ClosedChain<F, C, I>> = Box::from_raw(typed_target);
                // _b is now dropped
            }
        }
    }

    fn wrap<F, C, I>(cc: ClosedChain<F, C, I>) -> Self where
    F: Future<Item = I, Error = io::Error>,
    C: Chain<F::Item, Output = T>,
    {
        ChainExecutor {
            poll_f: Self::poll_f::<F, C, I>,
            drop_f: Self::drop_f::<F, C, I>,
            target: Box::into_raw(Box::new(cc)) as *mut u8,
            _p: PhantomData,
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

trait FutureChain: Chain {
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
