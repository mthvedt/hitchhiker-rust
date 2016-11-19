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

/*
Why Chain is a bad idea

- Basically, we can only poll the 'top future'. We can't trigger more activity based on sub-futures.
- We force allocs unless we rewrite, like the entire mio stack.

Why Chain may not be a bad idea

- It hides future return types.
- Reduces the size of allocated future space.
- Proceeds immediately when the target is in cache.

Potential alternative

- Use Chain, but only for the main future. Spawn 'side' futures.
At any join point, we get either a future or an Item. We then choose to allocate a 'side future'
if we need.

*/

// struct VoidFuture<I, E>(PhantomData<(*const I, *const E)>);

// impl<I, E> Future for VoidFuture<I, E> {
//     type Item = I;
//     type Error = E;

//     fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
//         panic!("should be unreachable")
//     }
// }

// /// The result of a fn that may optionally return a future.
// pub enum FutureResult<F: Future> {
//     Ok(F::Item),
//     Err(F::Error),
//     Wait(F),
// }

// /// A state containing an Item, Error, or ClosedChain. Used as input to FutureChains.
// ///
// /// Ideally, ClosedChain should be paramterized by output and error type.
// /// I was unable to make this work without inducing ugly type signatures.
// /// So it's parameterized by (), (), and you have to side-effect returns and errors.
// pub enum ChainState<I, E> {
//     Ok(I),
//     Err(E),
//     Wait(ClosedChain),
// }

// // trait FutureChainHKT<I1, E1, I2, E2> {
// //     SelfType: FutureChain<I1, E1>,
// //     OtherType: FutureChain<I2, E2>,
// // }

// struct WaitingChain<F, C, I, E> {
//     f: F,
//     c: C,
//     _p: PhantomData<(*const I, *const E)>,
// }

// impl<F, C, I, E> WaitingChain<F, C, I, E> {
//     fn new(f: F, c: C) -> Self {
//         WaitingChain {
//             f: f,
//             c: c,
//             _p: PhantomData,
//         }
//     }
// }

// impl<F, C, I, E> Future for WaitingChain<F, C, I, E> where
// // I used to be an adventurer like you, until I took an Error to the E.
// F: Future<Item = I, Error = E>,
// C: FutureCont<I, E>,
// {
//     type Item = C::OutItem;
//     type Error = C::OutError;

//     fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
//         panic!()
//     }
// }

// // pub trait FutureChainLink<I, E> {
// //     type OutItem;
// //     type OutError;

// //     fn exec<F, Cont, Chain>(self, input: FutureResult<F>, cont: Cont, chain: Chain)
// //     -> ChainState<Chain::OutItem, Chain::OutError> where
// //     F: Future<Item = I, Error = E>,
// //     Cont: FutureCont<Self::OutItem, Self::OutError>,
// //     Chain: FutureChain<Cont::OutItem, Cont::OutError>;
// // }

// // struct OkLink<F, I, O> where
// // F: FnOnce(I) -> O {
// //     f: F,
// //     _p: PhantomData<(*const I, *const O)>,
// // }

// // impl<F, I, O> OkLink<F, I, O> {
// //     fn new(f: f) -> Self {
// //         OkLink {
// //             f: f,
// //             _p: PhantomData,
// //         }
// //     }
// // }

// // impl<F, I, O, E> FutureChainLink<I, E> for OkLink<F, I, O> {
// //     fn exec<F, Cont, Chain>(self, input: FutureResult<F>, cont: Cont, chain: Chain)
// //     -> ChainState<Chain::OutItem, Chain::OutError> where
// //     F: Future<Item = I, Error = E>,
// //     Cont: FutureCont<Self::OutItem, Self::OutError>,
// //     Chain: FutureChain<Cont::OutItem, Cont::OutError> {
// //         cont.exec()
// //     }
// // }

// pub trait FutureCont<I, E> {
//     type OutItem;
//     type OutError;

//     fn exec<C>(self, input: Result<I, E>, chain: C) -> ChainState<C::OutItem, C::OutError> where
//     C: FutureChain<Self::OutItem, Self::OutError>;
// }

// fn into_chain_state<F: Future>(i: FutureResult<F>) -> ChainState<F::Item, F::Error> {
//     panic!()
// }

// struct EmptyCont;

// impl<I, E> FutureCont<I, E> for EmptyCont {
//     type OutItem = I;
//     type OutError = E;

//     fn exec<C>(self, input: Result<I, E>, chain: C) -> ChainState<C::OutItem, C::OutError> where
//     C: FutureChain<Self::OutItem, Self::OutError>
//     {
//         match input {
//             Ok(i) => chain.exec_ok(i),
//             Err(e) => chain.exec_err(e),
//         }
//     }
// }

// struct ResultLink<F, I, E, OI, OE> {
//     f: F,
//     _p: PhantomData<(*const I, *const E, *const OI, *const OE)>,
// }

// impl<F, I, E, OI, OE> ResultLink<F, I, E, OI, OE> {
//     fn new(f: F) -> Self {
//         ResultLink {
//             f: f,
//             _p: PhantomData,
//         }
//     }
// }

// impl<F, I, E, OI, OE> FutureCont<I, E> for ResultLink<F, I, E, OI, OE> where
// F: FnOnce(Result<I, E>) -> Result<OI, OE>
// {
//     type OutItem = OI;
//     type OutError = OE;

//     fn exec<C>(self, input: Result<I, E>, chain: C) -> ChainState<C::OutItem, C::OutError> where
//     C: FutureChain<Self::OutItem, Self::OutError>
//     {
//         EmptyCont.exec((self.f)(input), chain)
//     }
// }

// struct EmptyChain;

// impl<I, E> FutureChain<I, E> for EmptyChain {
//     type OutItem = I;
//     type OutError = E;
//     type First = EmptyCont;
//     type Second = EmptyChain;

//     fn exec<F: Future<Item = I, Error = E>>(self, i: FutureResult<F>) -> ChainState<I, E> {
//         into_chain_state(i)
//     }
// }

// pub trait FutureChain<I, E>: Sized {
//     type OutItem;
//     type OutError;
//     type First: FutureCont<I, E>;
//     type Second: FutureChain<
//     <Self::First as FutureCont<I, E>>::OutItem,
//     <Self::First as FutureCont<I, E>>::OutError,
//     OutItem = Self::OutItem, OutError = Self::OutError
//     >;

//     fn exec<F: Future<Item = I, Error = E>>(self, i: FutureResult<F>) -> ChainState<Self::OutItem, Self::OutError>;

//     // /// Runs this FutureChain with the given continuation. When invoked by end users,
//     // /// often this is the identity chain.
//     // /// When invoked by library writers, often this is another FutureChain waiting on a different future.
//     // fn run_cont<L: FutureLambda<I, E>>(c: L) -> L::Out;

//     /// Execute this FutureChain with the given item.
//     fn exec_ok(self, i: I) -> ChainState<Self::OutItem, Self::OutError> {
//         self.exec::<VoidFuture<I, E>>(FutureResult::Ok(i))
//     }

//     /// Execute this FutureChain with the given error. The default behavior is to return the error immediately.
//     fn exec_err(self, e: E) -> ChainState<Self::OutItem, Self::OutError> {
//         self.exec::<VoidFuture<I, E>>(FutureResult::Err(e))
//     }

//     /// Executes this FutureChain with the given Future. The default behavior is to return a ClosedChain
//     /// wrapping the given Future.
//     fn wait<F: Future<Item = I, Error = E>>(self, f: F) -> ChainState<Self::OutItem, Self::OutError> {
//         self.exec(FutureResult::Wait(f))
//     }

//     // type Second: FutureLambda<
//     // <Self::First as FutureCont<I, E>>::OutItem,
//     // <Self::First as FutureCont<I, E>>::OutError,
//     // Out = Self::Out,
//     // >;

//     // /// Create a FutureChain the execution of which may yield to the given Future.
//     // fn join<F, J, I2>(self, f: F, j: J) -> Join<F, Self, J, I2, I, E> where
//     // F: Future<Error = E>,
//     // J: FnOnce(I2, F::Item) -> I,
//     // {
//     //     Join {
//     //         f: f,
//     //         c: self,
//     //         j: j,
//     //         _p: PhantomData,
//     //     }
//     // }
// }

// struct FutureChainImpl<A, B, I, E, LI, LE> {
//     first: A,
//     second: B,
//     _p: PhantomData<(*const I, *const E, *const LI, *const LE)>,
// }

// impl<A, B, I, E, LI, LE> FutureChainImpl<A, B, I, E, LI, LE> {
//     fn new(a: A, b: B) -> Self {
//         FutureChainImpl {
//             first: a,
//             second: b,
//             _p: PhantomData,
//         }
//     }
// }

// impl<A, B, I, E, LI, LE> FutureChain<I, E> for FutureChainImpl<A, B, I, E, LI, LE> where
// A: FutureCont<I, E>,
// B: FutureChain<A::OutItem, A::OutError>,
// {
//     type OutItem = B::OutItem;
//     type OutError = B::OutError;
//     type First = A;
//     type Second = B;

//     fn exec<F: Future<Item = I, Error = E>>(self, i: FutureResult<F>) -> ChainState<Self::OutItem, Self::OutError> {
//         match i {
//             FutureResult::Ok(i) => self.first.exec(Ok(i), self.second),
//             FutureResult::Err(e) => self.first.exec(Err(e), self.second),
//             FutureResult::Wait(f) => panic!("TODO bundle up f, first, exec with second"),
//         }
//     }
// }

// pub fn bind<F, C, I, E, I2, E2>(f: F, c: C) -> impl FutureChain<I, E, OutItem = C::OutItem, OutError = C::OutError>
// where
// F: FnOnce(Result<I, E>) -> ChainState<C::OutItem, C::OutError>,
// C: FutureChain<I2, E2>,
// {
//     let (first, second) = c.split();

// }

// pub fn bind_fut<F, C, I, E>(f: F, c: C) -> impl FutureChain<I, E, OutItem = C::OutItem, OutError = C::OutError> where
// F: FutureCont<I, E>,
// C: FutureChain<F::OutItem, F::OutError>,
// {
//     // struct BindCont<F: FutureCont<I, E>, C: FutureChain<LI, LE>, I, E, LI, LE> {
//     //     f1: F,
//     //     f2: C::First,
//     //     _p: PhantomData<(*const I, *const E)>,
//     // }

//     // impl<F: FutureCont<I, E>, C: FutureChain<LI, LE>, I, E, LI, LE> FutureCont<I, E> for BindCont<F, C, I, E, LI, LE> {
//     //     type OutItem = C::OutItem;
//     //     type OutError = C::OutError;

//     //     fn run_with<Fx, Cx>(self, input: FutureResult<Fx>, c: Cx) -> ChainState<Cx::OutItem, Cx::OutError> where
//     //     Fx: Future<Item = I, Error = E>,
//     //     Cx: FutureChain<C::OutItem, C::OutError> {

//     //     }
//     // }

//     // let (f2, rest) = c.split();
//     // let fnew = BindCont::<F, C, I, E, LI, LE> {
//     //     f1: f,
//     //     f2: f2,
//     //     _p: PhantomData,
//     // };

//     panic!("this is wrong");
//     FutureChainImpl::new(f, c)
// }

// /// A 'nested Chain' that feeds into both a normal chain, C1, and a chain expecting a Future, C2.
// /// When given an Item or Err, it feeds that into the normal chain.
// /// When given a future F, it creates a future out of (F, C1) and feeds that into C2.
// // pub struct Joiner<NormCh, I, E, F, L, LErr, FutCh> where
// // NormCh: Chain<Result<I, E>, Out = FutureResult<F>>,
// // F: Future<Item = L, Error = LErr>,
// // FutCh: FutureLambda<L, LErr>,
// // {
// //     normal_chain: NormCh,
// //     future_chain: FutCh,
// //     _p: PhantomData<(*const I, *const E)>,
// // }

// // impl<NormCh, I, E, F, L, LErr, FutCh> FutureChain<I, E> for Joiner<NormCh, I, E, F, L, LErr, FutCh> where
// // NormCh: Chain<Result<I, E>, Out = FutureResult<F>>,
// // F: Future<Item = L, Error = LErr>,
// // FutCh: FutureLambda<L, LErr>,
// // {
// // }

// // pub fn join<F, C, I, E>(f: F, c: C) -> impl FutureChain<I, E> where
// // F: FnOnce(JoinChain) -> Foo,
// // C: FutureChain<I, E>,
// // {
// //     // Creates a future chain that feeds into another chain.
// //     // If the first chain ever blocks,
// // }

// // pub struct Join {
// //     outer: C1,
// //     inner: C2,
// //     joiner: Joiner,
// // }

// // impl FutureChain for Join {
// //     fn exec<F: Future<Item = I, Error = E>>(self, r: FutureResult<F>) -> Self::Out {
// //         let outer_result = match r {
// //             FutureResult::Ok(i) => self.outer.exec(i),
// //             FutureResult::Err(e) => self.outer.exec(e),
// //             FutureResult::Wait(f) => return self.inner.exec(TypedClosedChain::new(f, self.inner)),
// //         };

// //         match outer_result {
// //             Ok(i) => self.inner.exec_ok(i),
// //             Err(e) => self.inner.exec_err(e),
// //         }
// //         // Return a future joining on this FutureChain and the result of the given Future.
// //     }
// // }

// // fn join<F, C>(f: F, c: C, j: J) {

// // }

// // impl<I, E, C> FutureChain<I, E> for C where
// // C: Chain<ChainState<I, E>, Out = Option<ClosedChain>> + 'static {
// // }

// // pub struct Join<F, C, J, I, O, E> where
// // F: Future<Error = E>,
// // C: FutureChain<O, E>,
// // J: FnOnce(I, F::Item) -> O,
// // I: 'static,
// // O: 'static,
// // E: 'static,
// // {
// //     f: F,
// //     c: C,
// //     j: J,
// //     _p: PhantomData<*const I>,
// // }

// // impl<F, C, J, I, O, E> Chain<ChainState<I, E>> for Join<F, C, J, I, O, E> where
// // F: Future<Error = E> + 'static,
// // C: FutureChain<O, E> + 'static,
// // J: FnOnce(I, F::Item) -> O + 'static,
// // I: 'static,
// // O: 'static,
// // E: 'static,
// // {
// //     type Out = Option<ClosedChain>;

// //     fn exec(mut self, i: ChainState<I, E>) -> Self::Out {
// //         match i {
// //             ChainState::Ok(i) => match self.f.poll() {
// //                 Ok(Async::Ready(fi)) => self.c.exec_ok((self.j)(i, fi)),
// //                 Ok(Async::NotReady) => {
// //                     let c = self.c;
// //                     let j = self.j;
// //                     Some(ClosedChain::new(self.f, premap_ok(move |i2| (j)(i, i2), c)))
// //                 },
// //                 Err(e) => self.c.exec_err(e),
// //             },
// //             ChainState::Err(e) => self.c.exec_err(e),
// //             ChainState::Wait(cc) => Some(cc),
// //         }
// //     }
// // }

// // /// Like bind, but for a fn that is only interested in Ok(item) results.
// // /// If given an Err or Wait, it is passed to the Chain directly.
// // pub fn bind_ok<F, C, I, E, L>(link: F, c: C) -> impl FutureChain<I, E> where
// // F: FnOnce(I, C) -> C::Out + 'static,
// // C: FutureChain<L, E>,
// // I: 'static,
// // E: 'static,
// // L: 'static,
// // {
// //     let mylink = |fcr, c2| match fcr {
// //         ChainState::Ok(i) => (link)(i, c2),
// //         ChainState::Err(e) => c2.exec_err(e),
// //         ChainState::Wait(s) => c2.exec(ChainState::Wait(s)),
// //     };

// //     bind(mylink, c)
// // }

// // pub fn premap_ok<F, C, I, E, O>(f: F, c: C) -> impl FutureChain<I, E> where
// // F: FnOnce(I) -> O + 'static,
// // C: FutureChain<O, E>,
// // I: 'static,
// // E: 'static,
// // O: 'static,
// // {
// //     bind_ok(|i, c| c.exec_ok(f(i)), c)
// // }

// // /// Like bind, but for a fn that is only interested in Err(e) results.
// // /// If given an Ok or Wait, it is passed to the Chain directly.
// // pub fn bind_catch<F, C, I, E, L>(catch: F, c: C) -> impl FutureChain<I, E> where
// // F: FnOnce(E, C) -> C::Out + 'static,
// // C: FutureChain<I, L>,
// // I: 'static,
// // E: 'static,
// // L: 'static,
// // {
// //     let mycatch = |fcr, c2: C| match fcr {
// //         ChainState::Ok(i) => c2.exec_ok(i),
// //         ChainState::Err(e) => (catch)(e, c2),
// //         ChainState::Wait(s) => c2.exec(ChainState::Wait(s)),
// //     };

// //     bind(mycatch, c)
// // }

// trait ClosedChainInner {
//     fn poll(&mut self) -> Option<ClosedChain>;
// }

// struct ClosedChainInnerImpl<F, C> where
// F: Future,
// C: FutureChain<F::Item, F::Error>,
// {
//     b: Option<(F, C)>,
// }

// impl<F, C> ClosedChainInnerImpl<F, C> where
// F: Future,
// C: FutureChain<F::Item, F::Error>,
// {
//     fn new(f: F, chain: C) -> Self {
//         ClosedChainInnerImpl {
//             b: Some((f, chain)),
//         }
//     }
// }

// impl<F, C> ClosedChainInner for ClosedChainInnerImpl<F, C> where
// F: Future + 'static,
// C: FutureChain<F::Item, F::Error>,
// {
//     fn poll(&mut self) -> Option<ClosedChain> {
//         let unbox = match mem::replace(&mut self.b, None) {
//             Some(tuple) => tuple,
//             None => panic!("cannot poll a used Future"),
//         };

//         let (mut f, link) = unbox;

//         // TODO: This can be slightly optimized by using a single indirection,
//         // at the expense of having to store a function pointer for the drop function.
//         match f.poll() {
//             Ok(Async::Ready(i)) => link.exec_ok(i),
//             Ok(Async::NotReady) => Some(ClosedChain::wrap(Self::new(f, link))),
//             Err(e) => link.exec_err(e),
//         }
//     }
// }

// pub struct ClosedChain {
//     inner: Box<ClosedChainInner>,
// }

// impl ClosedChain {
//     fn new<F, C>(f: F, chain: C) -> Self where
//     F: Future + 'static,
//     C: FutureChain<F::Item, F::Error> + 'static,
//     {
//         Self::wrap(ClosedChainInnerImpl::new(f, chain))
//     }

//     fn wrap<F, C>(cc: ClosedChainInnerImpl<F, C>) -> Self where
//     F: Future + 'static,
//     C: FutureChain<F::Item, F::Error> + 'static,
//     {
//         ClosedChain {
//             inner: Box::new(cc),
//         }
//     }
// }

// impl Future for ClosedChain {
//     type Item = ();
//     type Error = ();

//     fn poll(&mut self) -> Poll<(), ()> {
//         match self.inner.poll() {
//             Some(mut cc) => {
//                 mem::swap(self, &mut cc);
//                 Ok(Async::NotReady)
//             },
//             None => Ok(Async::Ready(())),
//         }
//     }
// }

// // TODO: below might be faster... Who knows.

// // pub struct ClosedChain {
// //     poll_f: fn(*mut u8),
// //     drop_f: fn(*mut u8),
// //     target: *mut u8,
// // }

// // impl ClosedChain {
// //     fn new<F: Future, C: FutureChain<F::Item, F::Error>>(f: F, chain: C) -> Self {
// //         Self::wrap(ClosedChainInner::new(f, chain))
// //     }

// //     /// Internal poll function. This consumes the target pointer; the caller must set it to null before calling.
// //     fn poll_f<F, C>(target: *mut u8) where
// //     F: Future,
// //     C: FutureChain<F::Item, F::Error>,
// //     {
// //         unsafe {
// //             let typed_target = mem::transmute(target);
// //             let target_box: Box<ClosedChainInner<F, C>> = Box::from_raw(typed_target);
// //             target_box.poll()
// //         }
// //     }

// //     /// Internal drop function, called by <Self as Drop>::drop.
// //     fn drop_f<F, C>(target: *mut u8) where
// //     F: Future,
// //     C: FutureChain<F::Item, F::Error>,
// //     {
// //          unsafe {
// //             let typed_target = mem::transmute(target);
// //             let _b: Box<ClosedChainInner<F, C>> = Box::from_raw(typed_target);
// //             // _b is now dropped
// //         }
// //     }

// //     fn wrap<F, C>(cc: ClosedChainInner<F, C>) -> Self where
// //     F: Future,
// //     C: FutureChain<F::Item, F::Error>,
// //     {
// //         ClosedChain {
// //             poll_f: Self::poll_f::<F, C>,
// //             drop_f: Self::drop_f::<F, C>,
// //             target: Box::into_raw(Box::new(cc)) as *mut u8,
// //         }
// //     }

// //     fn poll(mut self) {
// //         let target = self.target;
// //         self.target = ptr::null::<u8>() as *mut _;
// //         (self.poll_f)(target)
// //     }
// // }

// // impl Drop for ClosedChain {
// //     fn drop(&mut self) {
// //         if self.target as *const _ != ptr::null() {
// //             // Put the null check in the static fn, so it's not hidden behind an indirection.
// //             // Then it will be optimized away most of the time.
// //             (self.drop_f)(self.target);
// //         }
// //         // Not needed, but for completeness' sake
// //         mem::drop(self.poll_f);
// //         mem::drop(self.drop_f);
// //     }
// // }
