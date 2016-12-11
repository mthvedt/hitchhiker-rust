#![cfg_attr(test, feature(test))]

extern crate futures;
extern crate httparse;
#[macro_use]
extern crate log;
extern crate tokio_core;
extern crate tokio_proto;
extern crate tokio_service;

extern crate thunderhead_store;

mod http;

mod react;

mod server;
pub use server::*;

mod thunderhead;

pub mod util;

#[cfg(test)]
extern crate env_logger;
#[cfg(test)]
#[macro_use]
extern crate lazy_static;
#[cfg(test)]
extern crate test;

#[cfg(test)]
mod tests;
