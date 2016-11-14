use std::marker::PhantomData;

use std::mem::{drop, forget, replace, swap, transmute, uninitialized};

use futures::{Async, Future, Poll};

use tdfuture::future::FutureResult;

/// The internal state of a ChainFuture.
struct ChainState {
    /// A pointer to the next pollable future belonging to the next link to poll.
    poller: *mut u8,
    /// The next link to poll.
    link_pointer: *mut u8,
    /// Function pointer to the poll_with_state function of link_pointer.
    link_poll_pointer: *mut u8,
}

// TODO: uncomment, test and bench
trait FutureChainLink {
    type Input;
    type Output;
    type InputError;
    type OutputError;

    /// When called, poller.link_pointer must point to self.
    fn poll_with_state(&mut self, poller: &mut ChainState) -> Poll<Self::Output, Self::OutputError>;

    fn continue_with_state(&mut self, i: Result<Self::Input, Self::InputError>, poller: &mut ChainState)
    -> Poll<Self::Output, Self::OutputError>;

    fn drop_unexecuted(&mut self);
}

struct IntermediateChainLink<Input, InputError, BlockingFuture, InputTwo, InputErrorTwo, Cont, NextLink> where
Cont: FnOnce(Result<Input, InputError>) -> FutureResult<BlockingFuture>,
BlockingFuture: Future<Item = InputTwo, Error = InputErrorTwo>,
NextLink: FutureChainLink<Input = InputTwo, InputError = InputErrorTwo>,
{
    cont: Cont,
    next_link: NextLink,
    _phantom: PhantomData<(Input, InputError, BlockingFuture)>,
}

impl<Input, InputError, BlockingFuture, InputTwo, InputErrorTwo, Cont, NextLink> FutureChainLink for
IntermediateChainLink<Input, InputError, BlockingFuture, InputTwo, InputErrorTwo, Cont, NextLink> where
Cont: FnOnce(Result<Input, InputError>) -> FutureResult<BlockingFuture>,
BlockingFuture: Future<Item = InputTwo, Error = InputErrorTwo>,
NextLink: FutureChainLink<Input = InputTwo, InputError = InputErrorTwo>,
{
    type Input = Input;
    type InputError = InputError;
    type Output = NextLink::Output;
    type OutputError = NextLink::OutputError;

    fn poll_with_state(&mut self, state: &mut ChainState) -> Poll<NextLink::Output, NextLink::OutputError> {
        let blocking_future;

        unsafe {
            let p: *mut BlockingFuture = transmute(state.poller);
            blocking_future = &mut *p;
        }

        match blocking_future.poll() {
            Ok(Async::Ready(i)) => {
                drop(blocking_future);
                self.next_link.continue_with_state(Ok(i), state)
            },
            Ok(Async::NotReady) => unsafe {
                state.link_pointer = transmute(self as *mut _);
                state.link_poll_pointer = transmute(&Self::poll_with_state as *const _);

                Ok(Async::NotReady)
            },
            Err(e) => {
                drop(blocking_future);
                self.next_link.continue_with_state(Err(e), state)
            }
        }
    }

    /// state.poller must be uninitialized or dropped when this is called.
    fn continue_with_state(&mut self, input: Result<Input, InputError>, state: &mut ChainState) ->
    Poll<NextLink::Output, NextLink::OutputError> {
        let mut contmoved = unsafe { uninitialized() };
        swap(&mut self.cont, &mut contmoved);

        match (contmoved)(input) {
            FutureResult::Ok(item) => self.next_link.continue_with_state(Ok(item), state),
            FutureResult::Err(e) => self.next_link.continue_with_state(Err(e), state),
            FutureResult::Wait(f) => unsafe {
                state.link_pointer = transmute(self as *mut _);
                state.link_poll_pointer = transmute(&Self::poll_with_state as *const _);
                forget(replace(transmute(state.poller), f));

                Ok(Async::NotReady)
            }
        }
    }

    fn drop_unexecuted(&mut self) {
        drop(&mut self.cont);
        drop(&mut self.next_link);
        drop(&mut self._phantom);
    }
}

impl<Input, InputError, BlockingFuture, InputTwo, InputErrorTwo, Cont, NextLink> Drop for
IntermediateChainLink<Input, InputError, BlockingFuture, InputTwo, InputErrorTwo, Cont, NextLink> where
Cont: FnOnce(Result<Input, InputError>) -> FutureResult<BlockingFuture>,
BlockingFuture: Future<Item = InputTwo, Error = InputErrorTwo>,
NextLink: FutureChainLink<Input = InputTwo, InputError = InputErrorTwo>,
{
    fn drop(&mut self) {
        self.next_link.drop_unexecuted();
        drop(&mut self._phantom);
    }
}

struct FinalChainLink<Input, InputError, Output, OutputError, Cont, BlockingFuture> where
Cont: FnOnce(Result<Input, InputError>) -> FutureResult<BlockingFuture>,
BlockingFuture: Future<Item = Output, Error = OutputError>,
{
    cont: Cont,
    _phantom: PhantomData<(Input, InputError, Output, OutputError, BlockingFuture)>,
}

impl<Input, InputError, Output, OutputError, Cont, BlockingFuture> FutureChainLink for
FinalChainLink<Input, InputError, Output, OutputError, Cont, BlockingFuture> where
Cont: FnOnce(Result<Input, InputError>) -> FutureResult<BlockingFuture>,
BlockingFuture: Future<Item = Output, Error = OutputError>,
{
    type Input = Input;
    type InputError = InputError;
    type Output = BlockingFuture::Item;
    type OutputError = BlockingFuture::Error;

    /// Polls this chain link, returning the pointers (next chain link, poll function)
    /// where poll function is the poll function of the next chain link.
    /// The input poller must point to a BlockingFuture, and it is overwritten with the BlockingFuture
    /// of the next chain link.
    fn poll_with_state(&mut self, state: &mut ChainState) ->
    Poll<BlockingFuture::Item, BlockingFuture::Error> {
        let blocking_future;

        unsafe {
            let p: *mut BlockingFuture = transmute(state.poller);
            blocking_future = &mut *p;
        }

        blocking_future.poll()
    }

    /// state.poller must be uninitialized or dropped when this is called.
    fn continue_with_state(&mut self, input: Result<Input, InputError>, state: &mut ChainState) ->
    Poll<BlockingFuture::Item, BlockingFuture::Error> {
        let mut contmoved = unsafe { uninitialized() };
        swap(&mut self.cont, &mut contmoved);

        match (contmoved)(input) {
            FutureResult::Ok(item) => Ok(Async::Ready(item)),
            FutureResult::Err(e) => Err(e),
            FutureResult::Wait(f) => unsafe {
                forget(replace(transmute(state.poller), f));
                state.link_pointer = transmute(self as *mut _);
                state.link_poll_pointer = transmute(&Self::poll_with_state as *const _);

                Ok(Async::NotReady)
            }
        }
    }

    fn drop_unexecuted(&mut self) {
        drop(&mut self.cont);
        drop(&mut self._phantom);
    }
}

impl<Input, InputError, Output, OutputError, Cont, BlockingFuture> Drop for
FinalChainLink<Input, InputError, Output, OutputError, Cont, BlockingFuture> where
Cont: FnOnce(Result<Input, InputError>) -> FutureResult<BlockingFuture>,
BlockingFuture: Future<Item = Output, Error = OutputError>,
{
    fn drop(&mut self) {
        drop(&mut self._phantom);
    }
}
