//! Thunderhead library for futures.

// chain_future: totally unsafe and untested!
mod chain_future;

mod future;
pub use self::future::*;

mod loops;
pub use self::loops::*;

#[macro_use]
pub mod phkt;

mod spin;
pub use self::spin::*;
