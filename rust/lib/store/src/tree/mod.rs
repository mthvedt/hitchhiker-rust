// quickcheck_macros is currently broken, and we don't use it anyway
// #![cfg_attr(test, feature(plugin))]
// #![cfg_attr(test, plugin(quickcheck_macros))]

// #[cfg(test)]
// #[macro_use]
// extern crate quickcheck;

mod bucket;

mod counter;
pub use self::counter::*;

mod hotnode;
mod nodeptr;
mod util;

mod traits;
pub use self::traits::*;

pub mod btree;
// TODO why don't this work?
// pub use self::btree::*;

// TODO better isolation
#[macro_use]
pub mod testlib;

#[cfg(test)]
mod tests;
