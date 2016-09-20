// quickcheck_macros is currently broken, and we don't use it anyway
// #![cfg_attr(test, feature(plugin))]
// #![cfg_attr(test, plugin(quickcheck_macros))]

// #[cfg(test)]
// #[macro_use]
// extern crate quickcheck;

mod btree;
// TODO why don't this work?
// pub use self::btree::*;

#[cfg(test)]
mod tests;

extern crate test;
