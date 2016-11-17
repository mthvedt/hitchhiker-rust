#![feature(conservative_impl_trait)]
#![feature(test)]

//! This whole crate is attic'd.

extern crate test;

extern crate futures;

pub mod _impl;

mod chain;
pub use chain::*;

pub mod future;
