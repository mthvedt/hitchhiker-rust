#![feature(conservative_impl_trait)]
#![feature(try_from)]

// For benchmarks
#![cfg_attr(test, feature(test))]

extern crate byteorder;
extern crate futures;

// TODO consider better way to hide these macros...
#[macro_use]
mod data;
mod memory;
mod traits;
mod tree;

// TODO: cleaner separation of interfaces
pub use data::*;
pub use memory::*;
pub use traits::*;
