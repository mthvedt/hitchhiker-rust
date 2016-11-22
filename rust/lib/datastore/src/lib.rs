#![feature(proc_macro)]
#![feature(unique)]

#![cfg_attr(test, feature(plugin))]
#![cfg_attr(test, plugin(quickcheck_macros))]

extern crate bincode;
extern crate futures;
extern crate js;
#[macro_use]
extern crate lazy_static;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

extern crate thunderhead_store;

#[cfg(test)]
extern crate quickcheck;
#[cfg(test)]
extern crate quickcheck_macros;

// TODO flatten
mod datatype;
// mod engine;
pub mod engine;
