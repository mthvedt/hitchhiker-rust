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

// TODO we don't really need this
extern crate byteorder;
extern crate futures;

// TODO consider better way to hide these macros...
#[macro_use]
pub mod bench;
// TODO do we have data macros?
#[macro_use]
mod data;
mod memory;
mod traits;
pub mod tree;

// TODO: cleaner separation of interfaces
pub use data::*;
pub use memory::*;
pub use traits::*;
