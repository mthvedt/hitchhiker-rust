//! A module for various data types shared across other modules.

// Private modules
mod rcslice;

// Public modules
mod rcbytes;
pub use self::rcbytes::*;

mod slice;
pub use self::slice::*;

mod traits;
pub use self::traits::*;

// Not re-exported; gets its own modules because of the clumsiness of macro_use and module namespaces.
#[macro_use]
pub mod util;
