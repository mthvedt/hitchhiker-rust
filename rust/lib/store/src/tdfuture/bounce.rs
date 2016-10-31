//! A boxed-continuaton state machine, implemented with futures.
//!
//! A bounce future is a boxed pair: (input future, continuation closure).
//! The continuation closure should be small, to prevent excessive copying.
//! This allows us to use futres-rs in a way where the number and order of Futures
//! executed in a chain is unbounded and arbitrary, including recursion
//! and mutual recursion.
//! The trade-off is now we must allocate every time the continuation future transitions.
//!
//! # The name `bounce_future`
//!
//! In CS, a trampoline is an apparatus for repeteadly executing closures.
//! It is often used to implement unbounded recursion.
//! In `bounce_future`, the `poll()` mechanism essentially becomes a trampoline,
//! repeatedly polling bounce_futures until termination.
