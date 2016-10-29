use std::borrow::{Borrow, BorrowMut};

/// A reference type whose lifetime may be tied to external circumstances. For example,
/// Scoped is used for objects allocated from Arenas (not yet implemented), or pinned
/// to cached items that may be evicted.
///
/// It doesn't have to be this way: any Borrow, for example, is also Scoped. Scoped is for
/// objects which are like Borrow, but possibly more restricted on when they can be borrowed.
pub trait Scoped<T: ?Sized> {
	/// Gets the referent object, if possible.
    fn get<X>(&self) -> Option<&T>;
}

/// Like `Scoped` but for mutable things.
///
/// Needs to be a separate trait due to trait coherence.
pub trait ScopedMut<T: ?Sized>: Scoped<T> {
	/// Mutably gets the referent object, if possible.
    fn get_mut<X>(&mut self) -> Option<&mut T>;
}

impl<B: ?Sized, T: ?Sized> Scoped<T> for B where B: Borrow<T> {
    fn get<X>(&self) -> Option<&T> {
    	Some(self.borrow())
    }
}

impl<B: ?Sized, T: ?Sized> ScopedMut<T> for B where B: BorrowMut<T> {
    fn get_mut<X>(&mut self) -> Option<&mut T> {
    	Some(self.borrow_mut())
    }
}
