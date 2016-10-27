use futures;
use futures::Future as _f;

/*
TODO: how does a future work?
-- A future is constructed.
-- The constructed future is spawned from a reactor loop.
TODO: what does the reactor loop do with the spawn?
TODO: what do nested reactor loops do? They register interest with the thread-local task.

-- It's combinator'd with other futures.
-- Poll is called, and it goes to the bottom.
-- The future is parked and returns NotReady?

-- So the final future is some initial future plus
*/

/// An unchecked error, returned by TdFutures. Generally represents an error that
/// the TdFuture could not handle.
pub struct UncheckedError;

pub enum Result<Item, Error> {
    Ok(Item),
    Err(Error),
    Wait(Future),
}

/// A BoxFuture wrapper that ...
///
/// The idea is to avoid long future chains. In particular, infinite recursion
/// cannot be modeled in futures-rs combinators, since they always allocate
/// enough space for the statically deepest possible future chain.
///
/// Note that this is inefficient when working with futures-rs's builtin task/loop libraries,
/// since they allocate on the heap anyway, and now we have double indirection.
pub struct Future {
    // Option<Box<...>> uses zero-sized optimization; no overhead
	inner: Option<Box<futures::Future<Item = Result<(), Future>, Error = UncheckedError>>>
}

/// Chains an ordinary future and a TdFuture into a TdFuture.
pub fn chain_td<T, E, F1, F2, FErr>(f1: F1, f2: F2, ferr: FErr) -> Future where
F1: futures::Future<Item = T, Error = E> + 'static,
F2: FnOnce(T) -> Result<(), Future> + 'static,
FErr: FnOnce(E) -> UncheckedError + 'static,
{
    Future {
        inner: Some(Box::new(f1.map_err(ferr).map(f2))),
    }
}
