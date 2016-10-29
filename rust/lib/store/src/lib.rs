//! # By-value semantics
//!
//! Because futures are heavily used in Thunderhead, we often require arguments have static lifetimes.
//! As such, we often use by-value semantics. Users should be careful to wrap up their arguments
//! in lightweight containers like Boxes.
//!
//! # Scoped
//!
//! Thunderhead frequently uses 'Scoped' arguments. When used as inputs, their lifetime should be tied
//! to the computation context. If they ever go out of scope Thunderhead will assume that the computation
//! should be canceled. When used as outputs, they are guaranteed to be scoped to the computational context.

// TODO remove every feature...
#![feature(associated_type_defaults)]
#![feature(conservative_impl_trait)]
// #![feature(duration_checked_ops)]
#![feature(try_from)]
#![feature(log_syntax)]
#![feature(trace_macros)]

// For benchmarks
// #![cfg_attr(test, feature(test))]
// TODO: isolate feature(test)
#![feature(test)]

extern crate typed_arena;
extern crate futures;

// Used for bench. TODO: separate crates...
extern crate rand;

pub mod alloc;

// TODO consider better way to hide these macros...
#[macro_use]
pub mod bench;
// TODO do we have data macros?
#[macro_use]
mod data;
mod traits;
mod tdfuture;
pub mod tree;

// TODO: cleaner separation of interfaces
pub use data::*;
pub use traits::*;
