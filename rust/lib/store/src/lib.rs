#![feature(conservative_impl_trait)]
#![feature(try_from)] 

extern crate byteorder;
extern crate futures;

mod data;
mod memory;
mod traits;

// TODO: cleaner separation of interfaces
pub use data::*;
pub use memory::*;
pub use traits::*;
