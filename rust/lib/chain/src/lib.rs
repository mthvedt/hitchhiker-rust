#![feature(conservative_impl_trait)]
#![feature(test)]

extern crate test;

extern crate futures;

pub mod _impl;

mod chain;
pub use chain::*;

pub mod future;
