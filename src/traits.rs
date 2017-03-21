use std::io;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use counter::Counter;

// Sketch for sync:
// TODO: rename this to sync

// Proximally, we just need a test interface.
// We would like it to become the main, sync, simple interface.
//
// We need to be able to read, write, and flush.
//
// TODO improve this.
// TODO: hide details
// TODO: consider location of support traits
#[derive(Debug)]
pub enum TreeError {
    EvalError,
    // TODO: distinguish by error role. EvalError, DbError &c
    IoError(io::Error),
    RuntimeError(String),
}

pub trait DerefSpec<'a> {
    type Target: ?Sized;
    type Deref: Deref<Target = Self::Target>;
}

// The purpose of the 'spec' pattern is we have to pass lifetimes to entries. Or do we?
// TODO: We might be able to get rid of this, because Get has to be lifetimed anyway.
pub trait MapSpec<'a> {
    type Entry: Entry<'a, Self>;
    type Value: ?Sized;

    // TODO: does this work?
    // TODO: set up cases to test that lifetimes behave as expected
    type GetSpec: for<'x> DerefSpec<'x, Target = Self::Value>;
}

pub trait Entry<'a, Spec: MapSpec<'a> + ?Sized> {
    /// Gets a reference to this Entry's value. This is at least as fast as calling read_value
    /// and discarding the read value, and can be faster depending on implementation.
    /// Implementations may copy or clone the read value.

    // TODO: can we make this 'a? Entry<'a> is a subtype of Entry<'a2> where a2 is shorter, right?
    // 'b is guaranteed to live less than or equal to 'a, but this is not enforced by the constraint checker
    fn get<'b>(&'b self) -> <Spec::GetSpec as DerefSpec<'b>>::Deref;

    /// Like get, but destroys this Entry. Useful if you want to return the reference
    /// while keeping the 'a lifetime.
    fn unwrap(self) -> <Spec::GetSpec as DerefSpec<'a>>::Deref;

    // /// Reads the value, copies or clones it, and returns it. This is at least as fast
    // /// as calling get and copying the value, and can be faster depending on implementation.
    // fn read(&self) -> Spec::Value where Spec::Value: Sized;
}

pub trait Cursor<'a, Spec: MapSpec<'a> + ?Sized>: Entry<'a, Spec> + Sized {
    fn exists(&self) -> bool;

	fn next(self) -> Result<Self, Self>;
}

/// A map where the keys are byte strings.
pub trait Map<Spec: for<'x> MapSpec<'x> + ?Sized> {
    // TODO: existence check
    fn entry<'a, K: AsRef<[u8]>>(&'a self, k: K) -> Result<Option<<Spec as MapSpec<'a>>::Entry>, TreeError>;

    fn get<'a, K: AsRef<[u8]>>(&'a self, k: K) -> Result<Option<<<Spec as MapSpec<'a>>::GetSpec as DerefSpec<'a>>::Deref>, TreeError> {
        // Use closures instead of methods for type inference
        self.entry(k).map(|x| x.map(|y| y.unwrap()))
    }

    // // TODO: ideally we don't have 'a. Anyway to make it go away?
    // fn read<'a, K>(&'a self, k: K) -> Result<Option<<Spec as MapSpec<'a>>::Value>, TreeError> where
    // K: AsRef<[u8]>,
    // <Spec as MapSpec<'a>>::Value: Sized,
    // {
    //     // Use closures instead of methods for type inference
    //     self.entry(k).map(|x| x.map(|y| y.read()))
    // }

	/// Debug method to check this data structures's invariants.
    /// Only available with the testlib feature.
	// TODO: feature-gate.
	fn check_invariants(&self);
}

pub trait TreeSpec<'a>: MapSpec<'a> {
    type Cursor: Cursor<'a, Self>;
    type SuffixSpec: for<'x> TreeSpec<'x>;
    type SuffixImpl: Tree<'a, Self::SuffixSpec>;
    type SubrangeSpec: for<'x> TreeSpec<'x>;
    type SubrangeImpl: Tree<'a, Self::SubrangeSpec>;
}

/// A tree where the keys are byte strings.
pub trait Tree<'a, Spec: for<'x> TreeSpec<'x> + ?Sized>: Map<Spec> {
    fn cursor<'b, K: AsRef<[u8]>>(&'b self, k: K) -> Result<<Spec as TreeSpec<'b>>::Cursor, TreeError>;

    // TODO: must this return self?
    /// Returns a suffix of this Tree, containing all key-value pairs prefixed by the given bytes.
    /// The keys in the returned Tree are suffices; the given prefix is ommitted from the keys of the returned Tree.
    ///
    /// For example, if your Tree contained the key [1, 2, 3], and you requested the suffix [1],
    /// its key in the suffix Tree would be [2, 3].
    ///
    /// N.B.: A suffix Tree is not the same as the computer-science concept of a Suffix Tree.
    ///
    /// This function returns Self. Ideally, we'd like to be able to return an arbitrary type of subtree,
    /// but this makes Rust's constraint checker behave oddly in some cases, particularly with subtraits.
    fn suffix<'b, K: AsRef<[u8]>>(&'b self, prefix: K) -> <Spec as TreeSpec<'b>>::SuffixImpl;

    fn subrange<'b, K1: AsRef<[u8]>, K2: AsRef<[u8]>>(&self, start: K1, end: K2) -> <Spec as TreeSpec<'b>>::SubrangeImpl;
}

// TODO: MapMut?

pub trait DerefMutSpec<'a> {
    type Target: ?Sized;
    type DerefMut: DerefMut<Target = Self::Target>;
}

pub trait TreeMutSpec<'a>: TreeSpec<'a> {
    type EntryMut: EntryMut<'a, Self>;
    type CursorMut: Cursor<'a, Self> + EntryMut<'a, Self>;
    type GetMut: DerefMut<Target = Self::Value>;
    type SuffixMutSpec: for<'x> TreeMutSpec<'x>;
    type SuffixMutImpl: TreeMut<'a, Self::SuffixMutSpec>;
    type SubrangeMutSpec: for<'x> TreeMutSpec<'x>;
    type SubrangeMutImpl: TreeMut<'a, Self::SubrangeMutSpec>;
}

// TODO: how to handle readable/writable?
pub trait EntryMut<'a, Spec: TreeMutSpec<'a> + ?Sized>: Entry<'a, Spec> {
    /// Gets a reference to this Entry's value. This is at least as fast as calling read_value,
    /// modifying that value, and calling set_value, and can be faster depending on implementation.
    /// Implementations may copy or clone the read value.
    fn get_mut(&mut self) -> Spec::GetMut;

    /// Sets this Entry's value. This is at least as fast as calling get_value_mut
    /// and overwriting the read value, and can be faster depending on implementation.
    fn set<V: AsRef<Spec::Value>>(&mut self, v: V);
}

// TODO: MapMut
/// A handle to a mutable tree with byte keys.
pub trait TreeMut<'a, Spec: for<'x> TreeMutSpec<'x> + ?Sized>: Tree<'a, Spec> {
    fn entry_mut<'b, K: AsRef<[u8]>>(&'b mut self, k: K) -> Result<Option<<Spec as TreeMutSpec<'b>>::EntryMut>, TreeError>;

    fn cursor_mut<'b, K: AsRef<[u8]>>(&'b mut self, k: K) -> Result<<Spec as TreeMutSpec<'b>>::CursorMut, TreeError>;

    fn get_mut<'b, K: AsRef<[u8]>>(&'b mut self, k: K) -> Result<Option<<Spec as TreeMutSpec<'b>>::GetMut>, TreeError> {
        self.entry_mut(k).map(|x| x.map(|mut y| y.get_mut()))
    }

    fn put<K: AsRef<[u8]>, V: AsRef<<Spec as MapSpec<'static>>::Value>>(&mut self, k: K, v: V) -> Result<(), TreeError>;

    fn suffix_mut<'b, K: AsRef<[u8]>>(&'b self, prefix: K) -> <Spec as TreeMutSpec<'b>>::SuffixMutImpl;

    fn subrange_mut<'b, K1: AsRef<[u8]>, K2: AsRef<[u8]>>(&'b self, start: K1, end: K2) -> <Spec as TreeMutSpec<'b>>::SubrangeMutImpl;
}

// // TODO: make these better. What's the dominant design pattern? What's the expected use case?
pub trait PersistentTreeSpec<'a, 'p>: TreeSpec<'a> {
    type TransientSpec: for<'x> TransientTreeSpec<'x, 'p>;
    type TransientImpl: TransientTree<'a, 'p, Self::TransientSpec>;
}

pub trait PersistentTree<'a, Spec: for<'x> PersistentTreeSpec<'x, 'a> + ?Sized>: Tree<'a, Spec> {
    fn transient<'b>(&'b self) -> <Spec as PersistentTreeSpec<'b, 'a>>::TransientImpl;
}

pub trait TransientTreeSpec<'a, 'p>: TreeMutSpec<'a> {
    type PersistentSpec: for<'x> PersistentTreeSpec<'x, 'p>;
    type PersistentImpl: PersistentTree<'p, Self::PersistentSpec>;
}

pub trait TransientTree<'a, 'p, Spec: for<'x> TransientTreeSpec<'x, 'p> + ?Sized>: TreeMut<'a, Spec> {
    fn persistent(&self) -> <Spec as TransientTreeSpec<'a, 'p>>::PersistentImpl;
}

pub trait HistoryTreeSpec<'a>: TreeSpec<'a> {
    type DiffSpec: for<'x> HistoryTreeSpec<'x>;
    type DiffImpl: HistoryTree<'a, Self::DiffSpec>;
}

pub trait HistoryTree<'a, Spec: for<'x> HistoryTreeSpec<'x> + ?Sized>: Tree<'a, Spec> {
    fn counter(&self) -> Counter;

    fn diff(&self, c: Counter) -> <Spec as HistoryTreeSpec<'a>>::DiffImpl;
}
