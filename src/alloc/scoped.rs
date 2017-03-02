use std::borrow::{Borrow, BorrowMut};

/// A reference type whose lifetime may be tied to external circumstances. For example,
/// Scoped is used for objects allocated from Arenas (not yet implemented), or pinned
/// to cached items that may be evicted.
pub trait Scoped<T: ?Sized> {
	/// Gets the referent object, if possible.
    fn get(&self) -> Option<&T>;
}

/// Like `Scoped` but for mutable things.
///
/// Needs to be a separate trait due to trait coherence.
pub trait ScopedMut<T: ?Sized>: Scoped<T> {
	/// Mutably gets the referent object, if possible.
    fn get_mut(&mut self) -> Option<&mut T>;
}

pub trait ScopedValue<T>: ScopedMut<T> {
    fn unwrap(self) -> Option<T>;
}

impl<B: ?Sized, T: ?Sized> Scoped<T> for B where B: Borrow<T> {
    fn get(&self) -> Option<&T> {
    	Some(self.borrow())
    }
}

impl<B: ?Sized, T: ?Sized> ScopedMut<T> for B where B: BorrowMut<T> {
    fn get_mut(&mut self) -> Option<&mut T> {
    	Some(self.borrow_mut())
    }
}

pub struct ScopedRef<T: 'static + ?Sized>(pub &'static T);

// Transitively gives us a Scoped
impl<T: 'static+ ?Sized> Borrow<T> for ScopedRef<T> {
    fn borrow(&self) -> &T {
        self.0
    }
}
