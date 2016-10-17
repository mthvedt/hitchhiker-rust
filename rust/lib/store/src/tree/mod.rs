// quickcheck_macros is currently broken, and we don't use it anyway
// #![cfg_attr(test, feature(plugin))]
// #![cfg_attr(test, plugin(quickcheck_macros))]

// #[cfg(test)]
// #[macro_use]
// extern crate quickcheck;

mod bucket;

mod counter;
pub use self::counter::Counter;

mod memnode;
mod node;
mod noderef;
mod util;

pub mod btree;
// TODO why don't this work?
// pub use self::btree::*;

// TODO better isolation
#[macro_use]
pub mod testlib;

#[cfg(test)]
mod tests;
