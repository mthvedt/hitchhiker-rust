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

// use core::{cmp, fmt, ops};
// use core::hash::{Hash, Hasher};
// use core::iter::Filter;
// use core::str::from_utf8;

// use alloc::rc::{Rc, Weak};
// use alloc::boxed::Box;

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

    /// Construct a new `RcSlice` that only points to elements at
    /// indices `lo` (inclusive) through `hi` (exclusive).
    ///
    /// This consumes `self` to avoid unnecessary reference-count
    /// modifications. Use `.clone()` if it is necessary to refer to
    /// `self` after calling this.
    ///
    /// # Panics
    ///
    /// Panics if `lo > hi` or if either are strictly greater than
    /// `self.len()`.
    pub fn slice(mut self, lo: usize, hi: usize) -> RcSlice<T> {
        self.data = &self[lo..hi];
        self
    }
    /// Construct a new `RcSlice` that only points to elements at
    /// indices up to `hi` (exclusive).
    ///
    /// This consumes `self` to avoid unnecessary reference-count
    /// modifications. Use `.clone()` if it is necessary to refer to
    /// `self` after calling this.
    ///
    /// # Panics
    ///
    /// Panics if `hi > self.len()`.
    pub fn slice_to(self, hi: usize) -> RcSlice<T> {
        self.slice(0, hi)
    }
    /// Construct a new `RcSlice` that only points to elements at
    /// indices starting at  `lo` (inclusive).
    ///
    /// This consumes `self` to avoid unnecessary reference-count
    /// modifications. Use `.clone()` if it is necessary to refer to
    /// `self` after calling this.
    ///
    /// # Panics
    ///
    /// Panics if `lo > self.len()`.
    pub fn slice_from(self, lo: usize) -> RcSlice<T> {
        let hi = self.len();
        self.slice(lo, hi)
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

// pub type RcStr = RcSlice<u8>;
// pub type WeakStr = WeakSlice<u8>;

// pub struct Split<P> where P: FnMut(char) -> bool {
//     v: RcStr,
//     pred: P,
//     finished: bool
// }

// impl<P> Iterator for Split<P> where P: FnMut(char) -> bool {
//     type Item = RcStr;

//     #[inline]
//     fn next(&mut self) -> Option<RcStr> {
//         if self.finished { return None; }

//         let idx =  {
//             let pred = &mut self.pred;
//             from_utf8(&*self.v).unwrap().chars().position(|x| pred(x))
//         };
//         match idx {
//             None => self.finish(),
//             Some(idx) => {
//                 let ret = Some(self.v.clone().slice_to(idx));
//                 self.v = self.v.clone().slice_from(idx + 1);
//                 ret
//             }
//         }
//     }

//     #[inline]
//     fn size_hint(&self) -> (usize, Option<usize>) {
//         if self.finished {
//             (0, Some(0))
//         } else {
//             (1, Some(self.v.len() + 1))
//         }
//     }
// }

// pub struct SplitWhitespace {
//     inner: Filter<Split<fn(char) -> bool>, fn(&RcStr) -> bool>,
// }

// impl Iterator for SplitWhitespace {
//     type Item = RcStr;

//     fn next(&mut self) -> Option<RcStr> { self.inner.next() }
// }

// /// An internal abstraction over the splitting iterators, so that
// /// splitn, splitn_mut etc can be implemented once.
// trait SplitIter: Iterator {
//     /// Mark the underlying iterator as complete, extracting the remaining
//     /// portion of the slice.
//     fn finish(&mut self) -> Option<Self::Item>;
// }

// impl<P> SplitIter for Split<P> where P: FnMut(char) -> bool {
//     #[inline]
//     fn finish(&mut self) -> Option<RcStr> {
//         if self.finished { None } else { self.finished = true; Some(self.v.clone()) }
//     }
// }

// impl RcStr {
//     pub fn split_whitespace(self) -> SplitWhitespace {
//         const WHITESPACE_TABLE: &'static [(char, char)] = &[
//             ('\u{9}', '\u{d}'), ('\u{20}', '\u{20}'), ('\u{85}', '\u{85}'), ('\u{a0}', '\u{a0}'),
//             ('\u{1680}', '\u{1680}'), ('\u{2000}', '\u{200a}'), ('\u{2028}', '\u{2029}'),
//             ('\u{202f}', '\u{202f}'), ('\u{205f}', '\u{205f}'), ('\u{3000}', '\u{3000}')
//         ];

//         fn in_whitespace_table(c: char) -> bool {
//             use core::cmp::Ordering::{Equal, Less, Greater};
//             use core::slice::SliceExt;
//             WHITESPACE_TABLE.binary_search_by(|&(lo,hi)| {
//                 if lo <= c && c <= hi { Equal }
//                 else if hi < c { Less }
//                 else { Greater }
//             }).is_ok()
//         }

//         fn is_whitespace(c: char) -> bool {
//             match c {
//                 ' ' | '\x09' ... '\x0d' => true,
//                 c if c > '\x7f' => in_whitespace_table(c),
//                 _ => false
//             }
//         }

//         fn is_not_empty(s: &RcStr) -> bool {
//             !s.is_empty()
//         }

//         SplitWhitespace {
//             inner: Split {
//                 v: self,
//                 pred: is_whitespace as fn(char) -> bool,
//                 finished: false,
//             }.filter(is_not_empty as fn(&RcStr) -> bool)
//         }
//     }
// }

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
    fn test_slice() {
        let x = RcSlice::new(Box::new([1, 2, 3]));
        let real = [1, 2, 3];
        for i in 0..(3 + 1) {
            for j in i..(3 + 1) {
                let slice: RcSlice<_> = x.clone().slice(i, j);
                assert_eq!(&*slice, &real[i..j]);
            }
            assert_eq!(&*x.clone().slice_to(i), &real[..i]);
            assert_eq!(&*x.clone().slice_from(i), &real[i..]);
        }
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

        let whole = RcSlice::new(Box::new([Foo(drop_flag.clone()), Foo(drop_flag.clone())]));
        let part = whole.slice(1, 2);
        drop(part);
        assert_eq!(drop_flag.get(), 2);
    }

    // #[test]
    // fn test_split_whitespace() {
    //     // bytes of "Hello world"
    //     let s = RcSlice::new(Box::new([72, 101, 108, 108, 111, 32, 119, 111, 114, 108, 100]));
    //     let mut it = s.split_whitespace();
    //     assert_eq!(&*it.next().unwrap(), "Hello".as_bytes());
    //     assert_eq!(&*it.next().unwrap(), "world".as_bytes());
    // }
}
