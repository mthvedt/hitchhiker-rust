use std::mem;

use futures::{Async, Future, Poll};

/// The result of a fn that may optionally return a future.
///
/// They are not futures by default. The motivation behind this is that we want to continue directly,
/// instead of using the future mechanism, when a result is ready immediately. Additionally,
/// we want to use the FutureChain (not yet implemented) mechanism.
///
/// It is easy to adapt
/// a FutureResult into a FutureResultFuture; because their memory layouts are the same, this is
/// usually efficient.
pub enum FutureResult<F: Future> {
    Ok(F::Item),
    Err(F::Error),
    Wait(F),
}

impl<F: Future> FutureResult<F> {
    pub fn map<FM: FutureMap<Input = F::Item>>(self, mut futuremap: FM) -> FutureResult<MapFuture<F, FM>> {
        match self {
            FutureResult::Ok(item) => FutureResult::Ok(futuremap.apply(item)),
            FutureResult::Err(e) => FutureResult::Err(e),
            FutureResult::Wait(f) => FutureResult::Wait(MapFuture::new(f, futuremap)),
        }
    }

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

pub trait FutureMap {
    type Input;
    type Output;

    fn apply(&mut self, i: Self::Input) -> Self::Output;
}

pub struct MapFuture<F: Future, FM: FutureMap<Input = F::Item>> {
    first: F,
    second: FM,
}

impl<F: Future, FM: FutureMap<Input = F::Item>> MapFuture<F, FM> {
    pub fn new(future: F, futuremap: FM) -> Self {
        MapFuture {
            first: future,
            second: futuremap,
        }
    }
}

impl<F: Future, FM: FutureMap<Input = F::Item>> Future for MapFuture<F, FM> {
    type Item = FM::Output;
    type Error = F::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.first.poll() {
            Ok(Async::Ready(x)) => Ok(Async::Ready(self.second.apply(x))),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err(e),
        }
    }
}
