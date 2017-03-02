// Copyright (c) 2014 Huon Wilson
// Copyright (c) 2016 Mike Thvedt

// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:

// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

//! A thread-local reference-counted slice type.
//!
//! This is copy pasted from an unmaintained package made by Huon Wilson.
//! It shouldn't be published; instead use data::bytes.

use std;

use std::boxed::Box;
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::{Rc, Weak};

/// A reference-counted slice type.
///
/// This is exactly like `&[T]` except without lifetimes, so the
/// allocation only disappears once all `RcSlice`s have disappeared.
///
/// NB. this can lead to applications effectively leaking memory if a
/// short subslice of a long `RcSlice` is held.

#[derive(Clone)]
pub struct RcSlice<T> {
    data: *const [T],
    counts: Rc<Box<[T]>>,
}

/// A non-owning reference-counted slice type.
///
/// This is to `RcSlice` as `std::rc::Weak` is to `std::rc::Rc`, and
/// allows one to have cyclic references without stopping memory from
/// being deallocated.

#[derive(Clone)]
pub struct WeakSlice<T> {
    data: *const [T],
    counts: Weak<Box<[T]>>,
}

impl<T> RcSlice<T> {
    /// Construct a new `RcSlice` containing the elements of `slice`.
    ///
    /// This reuses the allocation of `slice`.
    pub fn new(slice: Box<[T]>) -> RcSlice<T> {
        RcSlice {
            data: &*slice,
            counts: Rc::new(slice),
        }
    }

    /// Downgrade self into a weak slice.
    pub fn downgrade(&self) -> WeakSlice<T> {
        WeakSlice {
            data: self.data,
            counts: Rc::downgrade(&self.counts)
        }
    }
}

impl<T> Deref for RcSlice<T> {
    type Target = [T];
    fn deref<'a>(&'a self) -> &'a [T] {
        unsafe {&*self.data}
    }
}

impl<T> AsRef<[T]> for RcSlice<T> {
    fn as_ref(&self) -> &[T] { &**self }
}

impl<T: PartialEq> PartialEq for RcSlice<T> {
    fn eq(&self, other: &RcSlice<T>) -> bool { **self == **other }
    fn ne(&self, other: &RcSlice<T>) -> bool { **self != **other }
}

impl<T: Eq> Eq for RcSlice<T> {}

impl<T: PartialOrd> PartialOrd for RcSlice<T> {
    fn partial_cmp(&self, other: &RcSlice<T>) -> Option<Ordering> {
        (**self).partial_cmp(&**other)
    }
    fn lt(&self, other: &RcSlice<T>) -> bool { **self < **other }
    fn le(&self, other: &RcSlice<T>) -> bool { **self <= **other }
    fn gt(&self, other: &RcSlice<T>) -> bool { **self > **other }
    fn ge(&self, other: &RcSlice<T>) -> bool { **self >= **other }
}

impl<T: Ord> Ord for RcSlice<T> {
    fn cmp(&self, other: &RcSlice<T>) -> Ordering { (**self).cmp(&**other) }
}

impl<T: Hash> Hash for RcSlice<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Hash::hash(&**self, state)
    }
}

impl<T: Debug> Debug for RcSlice<T> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        Debug::fmt(&**self, f)
    }
}

impl<T> WeakSlice<T> {
    /// Attempt to upgrade `self` to a strongly-counted `RcSlice`.
    ///
    /// Returns `None` if this is not possible (the data has already
    /// been freed).
    pub fn upgrade(&self) -> Option<RcSlice<T>> {
        self.counts.upgrade().map(|counts| {
            RcSlice {
                data: self.data,
                counts: counts
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use super::{RcSlice, WeakSlice};
    use std::cell::Cell;
    use std::cmp::Ordering;

    #[test]
    fn clone() {
        let x = RcSlice::new(Box::new([Cell::new(false)]));
        let y = x.clone();

        assert_eq!(x[0].get(), false);
        assert_eq!(y[0].get(), false);

        x[0].set(true);
        assert_eq!(x[0].get(), true);
        assert_eq!(y[0].get(), true);
    }

    #[test]
    fn test_upgrade_downgrade() {
        let x = RcSlice::new(Box::new([1]));
        let y: WeakSlice<_> = x.downgrade();

        assert_eq!(y.upgrade(), Some(x.clone()));

        drop(x);

        assert!(y.upgrade().is_none())
    }

    #[test]
    fn test_total_cmp() {
        let x = RcSlice::new(Box::new([1, 2, 3]));
        let y = RcSlice::new(Box::new([1, 2, 3]));
        let z = RcSlice::new(Box::new([1, 2, 4]));
        assert_eq!(x, x);
        assert_eq!(x, y);
        assert!(x != z);
        assert!(y != z);

        assert!(x < z);
        assert!(x <= z);
        assert!(!(x > z));
        assert!(!(x >= z));

        assert!(!(z < x));
        assert!(!(z <= x));
        assert!(z > x);
        assert!(z >= x);

        assert_eq!(x.partial_cmp(&x), Some(Ordering::Equal));
        assert_eq!(x.partial_cmp(&y), Some(Ordering::Equal));
        assert_eq!(x.partial_cmp(&z), Some(Ordering::Less));
        assert_eq!(z.partial_cmp(&y), Some(Ordering::Greater));

        assert_eq!(x.cmp(&x), Ordering::Equal);
        assert_eq!(x.cmp(&y), Ordering::Equal);
        assert_eq!(x.cmp(&z), Ordering::Less);
        assert_eq!(z.cmp(&y), Ordering::Greater);
    }

    #[test]
    fn test_partial_cmp() {
        use std::f64;
        let x = RcSlice::new(Box::new([1.0, f64::NAN]));
        let y = RcSlice::new(Box::new([1.0, f64::NAN]));
        let z = RcSlice::new(Box::new([2.0, f64::NAN]));
        let w = RcSlice::new(Box::new([f64::NAN, 1.0]));
        assert!(!(x == y));
        assert!(x != y);

        assert!(!(x < y));
        assert!(!(x <= y));
        assert!(!(x > y));
        assert!(!(x >= y));

        assert!(x < z);
        assert!(x <= z);
        assert!(!(x > z));
        assert!(!(x >= z));

        assert!(!(z < w));
        assert!(!(z <= w));
        assert!(!(z > w));
        assert!(!(z >= w));

        assert_eq!(x.partial_cmp(&x), None);
        assert_eq!(x.partial_cmp(&y), None);
        assert_eq!(x.partial_cmp(&z), Some(Ordering::Less));
        assert_eq!(z.partial_cmp(&x), Some(Ordering::Greater));

        assert_eq!(x.partial_cmp(&w), None);
        assert_eq!(y.partial_cmp(&w), None);
        assert_eq!(z.partial_cmp(&w), None);
        assert_eq!(w.partial_cmp(&w), None);
    }

    #[test]
    fn test_show() {
        let x = RcSlice::new(Box::new([1, 2]));
        assert_eq!(format!("{:?}", x), "[1, 2]");

        let y: RcSlice<i32> = RcSlice::new(Box::new([]));
        assert_eq!(format!("{:?}", y), "[]");
    }

    #[test]
    fn test_drop() {
        let drop_flag = Rc::new(Cell::new(0));
        struct Foo(Rc<Cell<i32>>);

        impl Drop for Foo {
            fn drop(&mut self) {
                let n = self.0.get();
                self.0.set(n + 1);
            }
        }

        let whole = RcSlice::new(Box::new([Foo(drop_flag.clone()), Foo(drop_flag.clone())]));

        drop(whole);
        assert_eq!(drop_flag.get(), 2);

        drop_flag.set(0);
    }
}
