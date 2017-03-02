//! Right now, these are not used, they're just a design sketch.

use std::cell::UnsafeCell;
use std::mem::{forget, size_of, swap};
use std::ops::{Deref, DerefMut};
use std::rc::{Rc, Weak};

use typed_arena::Arena;

/// An object whose lifetime may be tied to the existence of a different 'owning' object.
pub trait Scoped {
    type T;

    /// Uses the owning object to get a reference to the contained object.
    fn get<X>(&self, owner: &X) -> &T;

    /// Uses the owning object to get a mutable reference to the contained object.
    fn get_mut<X>(&mut self, owner: &X) -> &mut T;
}

/// A model trait providing CheckPtrs. This comes in safe and unsafe versions.
// TODO: a whole pointer safety model?
pub trait CheckModel {
    /// The CheckPtr type.
    type Ptr: CheckPtr;

    /// Get a WeakPtr to the given value from this WeakModel.
    fn ptr<X>(x: &X) -> Self::Ptr;
}

/// A pointer to something that cannot be dereferenced. It can optionally be implemented
/// to help check pointer equality, to enforce runtime invariants.
pub trait CheckPtr: 'static + Send + Sized {
    /// A no-op, but optionally panics if the given object doesn't match the one this handle refers to.
    fn assert_eq<X>(&self, x: &X);
}

/// An allocated T that belongs to an instance of an Alloc. You must hold a reference to the Alloc
/// to access the information within.
///
/// TODO: Scoped pointers are a little fat when T is a slice. This only affects the bytes situation
/// since we don't use unsized slices otherwise. (Not yet anyway.)
pub struct CheckScoped<M: CheckModel, T: ?Sized> {
    /// Ptr handle, referring to the Alloc that created this Scoped.
    ptr: M::Ptr,

    /// The data in this Scoped.
    bytes: *mut T,
}

/// An allocator, backed by a quick-free Arena.
///
/// TODO: If/once we have multiple kinds of WeakAcquire, we might consider making this generic
/// on one particular kind of WeakAcquire, to make it easier for owning code to swap implementations.
pub trait Alloc<M: CheckModel, T> {
    /// Outside users should probably use one of the provided fns.
    fn alloc(&mut self, t: T) -> Scoped<M, T>;
}

/// An allocator for raw bytes, backed by a quick-free Arena.
///
/// Refactor point: Right now we only use byte allocators. This might change.
pub trait ByteAlloc<M: CheckModel> {
    /// Alloc some raw, uninitialized bytes.
    fn alloc_raw(&mut self, size: usize) -> CheckScoped<M, [u8]>;
}

impl<M: CheckModel, T: ?Sized> CheckScoped<M, T> {
    fn new<X>(owner: &X, t: *mut T) -> Self {
        CheckScoped {
            ptr: M::ptr(owner),
            bytes: t,
        }
    }

    /// Remaps this Scoped to point to the given allocator. Not usually called by client code,
    /// but useful for nested allocator implementations.
    fn remap<X>(&mut self, new_owner: &X) {
        self.ptr = M::ptr(new_owner);
    }

    fn get<X>(&self, owner: &X) -> &T {
        self.ptr.assert_eq(owner);

        unsafe {
            &*self.bytes
        }
    }

    // This is safe because we expose at most one &mut inner borrow while &mut self is borrowed.
    fn get_mut<X>(&mut self, owner: &X) -> &mut T {
        self.ptr.assert_eq(owner);

        unsafe {
            &mut *self.bytes
        }
    }
}

/// Helper for SafeAlloc.
pub struct SafeCheckPtr {
    inner: usize,
}

impl SafeCheckPtr {
    fn new<X>(x: &X) -> Self {
        SafeCheckPtr {
            inner: x as *const _ as usize,
        }
    }
}

impl CheckPtr for SafeCheckPtr {
    fn assert_eq<X>(&self, x: &X) {
        assert!(self.inner == x as *const _ as usize);
    }
}

/// A CheckModel that uses safe pointers.
pub struct SafeCheckModel;

impl CheckModel for SafeCheckModel  {
    type Ptr = SafeCheckPtr;

    fn ptr<X>(x: &X) -> Self::Ptr {
        SafeCheckPtr::new(x)
    }
}

/// A allocator with runtime checks enabled, returned by safe_alloc.
pub struct ArenaAlloc<T> {
    inner: Arena<T>,
}

impl<M: CheckModel, T> Alloc<M, T> for ArenaAlloc<T> {
    fn alloc(&mut self, t: T) -> CheckScoped<M, T> {
        CheckScoped::new(self, self.inner.alloc(t) as *mut _)
    }
}

pub struct ByteArenaAlloc {
    inner: Arena<u8>,
}

impl<M: CheckModel> ByteAlloc<M> for ByteArenaAlloc {
    fn alloc_raw(&mut self, size: usize) -> CheckScoped<M, [u8]> {
        unsafe {
            CheckScoped::new(self, self.inner.alloc_uninitialized(size))
        }
    }
}
