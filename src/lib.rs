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

extern crate futures;
extern crate typed_arena;

// #[cfg(test)]
// TODO isolate
extern crate bytebuffer;
#[cfg(test)]
extern crate test;

// TODO: isolate
// #[cfg(bench)]
extern crate rand;
#[cfg(bench)]
extern crate test;

pub mod alloc;

// TODO consider better way to hide these macros...
#[macro_use]
pub mod bench;

// TODO do we have data macros?
// TODO: cleaner separation of interfaces
#[macro_use]
pub mod data;
// TODO remove all data, incl. data::Range
pub use data::Range;

pub mod sync;

pub mod tdfuture;

// #[cfg(test)]
// TODO: isolate with a feature
// pub mod testlib;

mod traits;
pub use traits::*;

pub mod tree;
pub use tree::btree::*;
pub use tree::testlib;

pub mod util;
