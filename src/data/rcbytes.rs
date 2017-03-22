//! Reference-counted byte boxes. Necessary because Rust doesn't support `Rc<[u8]>` yet.
//!
//! These types are a little slow, suffering from double-pointer-indirection.

use data::rcslice::{RcSlice, WeakSlice};

use std::borrow::Borrow;
use std::ops::Deref;

use super::traits::*;

/// A reference-counted box of bytes. Similar to `Rc<[u8]>`.
///
/// TODO (low-priority): This is a little slow, because it internally uses fat pointers and double pointer indirection.
/// It might be worth making it faster.

#[derive(Clone)]
pub struct RcBytes {
    data: RcSlice<u8>,
}

impl RcBytes {
    pub fn new<B: Borrow<[u8]>>(b: B) -> RcBytes {
        RcBytes {
            data: RcSlice::new(b.borrow().to_vec().into_boxed_slice()),
        }
    }

    // TODO these are redundant
    pub fn from_key<K: Key + ?Sized>(k: &K) -> RcBytes {
        Self::new(k.bytes())
    }

    pub fn from_value<V: Datum>(v: &V) -> RcBytes {
        RcBytes {
            data: RcSlice::new(v.box_copy()),
        }
    }

    pub fn downgrade(&self) -> WeakBytes {
        WeakBytes {
            data: self.data.downgrade(),
        }
    }
}

impl Borrow<[u8]> for RcBytes {
    fn borrow(&self) -> &[u8] {
        self.deref()
    }
}

impl Deref for RcBytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.data.borrow()
    }
}

/// A weak reference to a reference-counted box of bytes. Similar to `Weak<[u8]>`.
///
/// See also `RcBytes`.

#[derive(Clone)]
pub struct WeakBytes {
    data: WeakSlice<u8>,
}

impl WeakBytes {
    pub fn upgrade(&self) -> RcBytes {
        RcBytes {
            // Should never fail to upgrade for our purposes.
            data: self.data.upgrade().unwrap(),
        }
    }
}
