use std::cell::{Ref, RefCell, RefMut};

// Re-export data::util
pub use data::util::*;

/// An uninstantiable type.
pub enum Void {}
