//! Allocators used in Thunderhead. These represent a per-task ability to allocate,
//! verify allocations are valid,
//! and efficiently free all allocations at the end of the task.

use std::cell::UnsafeCell;
use std::mem::{forget, size_of, swap};
use std::ops::{Deref, DerefMut};
use std::rc::{Rc, Weak};

use typed_arena::Arena;

/// Something for which weak handles can be acquired. Weak handles have no defined behavior,
/// but they may optionally implement behavior to help verify runtime invariants.
pub trait WeakAcquire {
    type Handle: WeakHandle;

    fn acquire(&self) -> Self::Handle;
}

/// A weak handle to something.
pub trait WeakHandle: 'static + Send + Sized {
    /// A no-op, but optionally panics if the given WeakAcquire doesn't match this one.
    fn alloc_assert<W: WeakAcquire>(&self, w: &W);
}

/// An allocated T that belongs to an instance of an Alloc. You must hold a reference to the Alloc
/// to access the information within.
///
/// TODO: Scoped pointers are a little fat when T is a slice. This only affects the bytes situation
/// since we don't use unsized slices otherwise. (Not yet anyway.)
pub struct Scoped<A: WeakAcquire, T: ?Sized> {
    /// Alloc handle. Shared among all instances of Scoped that use the same instance of an Alloc.
    handle: A::Handle,

    /// The data in this Scoped.
    bytes: *mut T,
}

/// An allocator, backed by a quick-free Arena.
///
/// TODO: If/once we have multiple kinds of WeakAcquire, we might consider making this generic
/// on one particular kind of WeakAcquire, to make it easier for owning code to swap implementations.
pub trait Alloc<T: 'static + Send>: Sized + WeakAcquire {
    /// Outside users should probably use one of the provided fns.
    fn alloc(&mut self, t: T) -> Scoped<Self, T>;
}

/// An allocator for raw bytes, backed by a quick-free Arena.
///
/// Refactor point: Right now we only use byte allocators. This might change.
pub trait ByteAlloc: Sized + WeakAcquire {
    /// Alloc some raw, uninitialized bytes.
    fn alloc_raw(&mut self, size: usize) -> Scoped<Self, [u8]>;
}

impl<A: WeakAcquire, T: ?Sized> Scoped<A, T> {
    fn new(a: &A, t: *mut T) -> Self {
        Scoped {
            handle: a.acquire(),
            bytes: t,
        }
    }

    fn get(&self, alloc: &A) -> &T {
        self.handle.alloc_assert(alloc);

        unsafe {
            &*self.bytes
        }
    }

    // This is safe because we expose at most one &mut inner borrow while &mut self is borrowed.
    fn get_mut(&mut self, alloc: &A) -> &mut T {
        self.handle.alloc_assert(alloc);

        unsafe {
            &mut *self.bytes
        }
    }
}

/// Helper for SafeAlloc.
pub struct SafeHandle {
    inner: usize,
}

impl SafeHandle {
    fn new<X>(x: &X) -> Self {
        SafeHandle {
            inner: x as *const _ as usize,
        }
    }
}

impl WeakHandle for SafeHandle {
    fn alloc_assert<W: WeakAcquire>(&self, w: &W) {
        assert!(self.inner == w as *const _ as usize);
    }
}

/// A allocator with runtime checks enabled, returned by safe_alloc.
pub struct SafeArenaAlloc<T: 'static + Send> {
    inner: Arena<T>,
}

impl<T: 'static + Send> WeakAcquire for SafeArenaAlloc<T> {
    type Handle = SafeHandle;

    fn acquire(&self) -> SafeHandle {
        SafeHandle::new(self)
    }
}

impl<T: 'static + Send> Alloc<T> for SafeArenaAlloc<T> {
    fn alloc(&mut self, t: T) -> Scoped<Self, T> {
        Scoped::new(self, self.inner.alloc(t) as *mut _)
    }
}

pub struct SafeByteArenaAlloc {
    inner: Arena<u8>,
}

impl WeakAcquire for SafeByteArenaAlloc {
    type Handle = SafeHandle;

    fn acquire(&self) -> SafeHandle {
        SafeHandle::new(self)
    }
}

impl ByteAlloc for SafeByteArenaAlloc {
    fn alloc_raw(&mut self, size: usize) -> Scoped<Self, [u8]> {
        unsafe {
            Scoped::new(self, self.inner.alloc_uninitialized(size))
        }
    }
}
