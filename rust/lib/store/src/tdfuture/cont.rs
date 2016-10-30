//! A boxed-continuaton state machine, implemented with futures.
//!
//! A cont future is a boxed pair: (input future, continuation closure).
//! The continuation closure should be small, to prevent excessive copying.
//! This allows us to use futres-rs in a way where the number and order of Futures
//! executed in a chain is unbounded and arbitrary, including recursion
//! and mutual recursion.
//! The trade-off is now we must allocate every time the continuation future transitions.
