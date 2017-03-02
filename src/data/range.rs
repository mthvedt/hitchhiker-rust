use std::borrow::Borrow;
use std::cmp::Ordering;

/// A range of keys, exclusive or inclusive.
///
/// TODO: consider using alloc.rs for these.
pub struct Range {
    open_left: bool,
    open_right: bool,
    // TODO: this representation is a little bit inefficient. And by "a little bit" I mean "quite a lot".
    left: Box<[u8]>,
    right: Box<[u8]>,
}

impl Range {
    pub fn open(left: Box<[u8]>, right: Box<[u8]>) -> Self {
        assert!(left < right);

        Range {
            open_left: true,
            open_right: true,
            left: left,
            right: right,
        }
    }

    pub fn left_open(left: Box<[u8]>, right: Box<[u8]>) -> Self {
        assert!(left < right);

        Range {
            open_left: true,
            open_right: false,
            left: left,
            right: right,
        }
    }

    pub fn right_open(left: Box<[u8]>, right: Box<[u8]>) -> Self {
        assert!(left < right);

        Range {
            open_left: false,
            open_right: true,
            left: left,
            right: right,
        }
    }

    /// N. B.: This is the only range constructor where left and right are allowed to be equal.
    pub fn closed(left: Box<[u8]>, right: Box<[u8]>) -> Self {
        assert!(left <= right);

        Range {
            open_left: false,
            open_right: false,
            left: left,
            right: right,
        }
    }

    pub fn contains<K: Borrow<[u8]>>(&self, k: &K) -> bool {
        match k.borrow().cmp(self.left.borrow()) {
            Ordering::Less => return false,
            Ordering::Equal => if self.open_left {
                return false
            } else {
                () // proceed to right comparison
            },
            Ordering::Greater => (), // proceed to right comparison
        }

        match k.borrow().cmp(self.right.borrow()) {
            Ordering::Less => true,
            Ordering::Equal => !self.open_right,
            Ordering::Greater => false,
        }
    }
}
