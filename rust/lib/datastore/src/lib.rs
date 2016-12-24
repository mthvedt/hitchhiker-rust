#![feature(integer_atomics)]
#![feature(proc_macro)]
#![feature(unique)]

#![feature(conservative_impl_trait)]

#![cfg_attr(test, feature(plugin))]
// #![cfg_attr(test, plugin(quickcheck_macros))]

extern crate bincode;
extern crate futures;
extern crate js;
extern crate libc;
#[macro_use]
extern crate lazy_static;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

extern crate thunderhead_store;

#[cfg(test)]
extern crate quickcheck;

// TODO flatten
pub mod datatype;
pub mod engine;
pub mod platform;
mod lens;
