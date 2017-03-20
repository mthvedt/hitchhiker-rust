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

pub trait MapSpec<'a> {
    type Entry: Entry<'a, Self>;
    type Value: ?Sized;
    type Get: Deref<Target = Self::Value> + 'a;
}

pub trait Entry<'a, Spec: MapSpec<'a> + ?Sized> {
    /// Gets a reference to this Entry's value. This is at least as fast as calling read_value
    /// and discarding the read value, and can be faster depending on implementation.
    /// Implementations may copy or clone the read value.
    fn get(&self) -> Spec::Get;

    // TODO: figure out interface
    /// Reads the value, copies or clones it, and returns it. This is at least as fast
    /// as calling get and copying the value, and can be faster depending on implementation.
    fn read(&self) -> Spec::Value where Spec::Value: Sized;
}

pub trait Cursor<'a, Spec: MapSpec<'a> + ?Sized>: Entry<'a, Spec> + Sized {
    fn exists(&self) -> bool;

	fn next(self) -> Result<Self, Self>;

    // TODO: do we want backwards cursors?
}

/// A map where the keys are byte strings.
pub trait Map<Spec: for<'a> MapSpec<'a> + ?Sized> {
    // TODO: existence check
    fn entry<'a, K: AsRef<[u8]>>(&'a self, k: K) -> Result<Option<<Spec as MapSpec<'a>>::Entry>, TreeError>;

    fn get<'a, K: AsRef<[u8]>>(&'a self, k: K) -> Result<Option<<Spec as MapSpec<'a>>::Get>, TreeError> {
        // Use closures instead of methods for type inference
        self.entry(k).map(|x| x.map(|y| y.get()))
    }

    // TODO: ideally we don't have 'a. Anyway to make it go away?
    fn read<'a, K>(&'a self, k: K) -> Result<Option<<Spec as MapSpec<'a>>::Value>, TreeError> where
    K: AsRef<[u8]>,
    <Spec as MapSpec<'a>>::Value: Sized,
    {
        // Use closures instead of methods for type inference
        self.entry(k).map(|x| x.map(|y| y.read()))
    }

	/// Debug method to check this data structures's invariants.
    /// Only available with the testlib feature.
	// TODO: feature-gate.
	fn check_invariants(&self);
}

pub trait TreeSpec<'a>: MapSpec<'a> {
    type Cursor: Cursor<'a, Self>;
    type SuffixSpec: for<'b> TreeSpec<'b>;
    type SuffixImpl: Tree<Self::SuffixSpec>;
    type SubrangeSpec: for<'b> TreeSpec<'b>;
    type SubrangeImpl: Tree<Self::SubrangeSpec>;
}

/// A tree where the keys are byte strings.
pub trait Tree<Spec: for<'a> TreeSpec<'a> + ?Sized>: Map<Spec> {
    fn cursor<'a, K: AsRef<[u8]>>(&'a self, k: K) -> Result<<Spec as TreeSpec<'a>>::Cursor, TreeError>;

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
    fn suffix<'a, K: AsRef<[u8]>>(&'a self, prefix: K) -> <Spec as TreeSpec<'a>>::SuffixImpl;

    fn subrange<'a, K1: AsRef<[u8]>, K2: AsRef<[u8]>>(&self, start: K1, end: K2) -> <Spec as TreeSpec<'a>>::SubrangeImpl;
}

// TODO: MapMut?

pub trait TreeMutSpec<'a>: TreeSpec<'a> {
    type EntryMut: EntryMut<'a, Self>;
    type CursorMut: Cursor<'a, Self> + EntryMut<'a, Self>;
    type GetMut: DerefMut<Target = Self::Value> + 'a;
    type SuffixSpecMut: for<'b> TreeMutSpec<'b>;
    type SuffixImplMut: TreeMut<Self::SuffixSpecMut>;
    type SubrangeSpecMut: for<'b> TreeMutSpec<'b>;
    type SubrangeImplMut: TreeMut<Self::SubrangeSpecMut>;
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
pub trait TreeMut<Spec: for<'a> TreeMutSpec<'a> + ?Sized>: Tree<Spec> {
    fn entry_mut<'a, K: AsRef<[u8]>>(&'a mut self, k: K) -> Result<Option<<Spec as TreeMutSpec<'a>>::EntryMut>, TreeError>;

    fn cursor_mut<'a, K: AsRef<[u8]>>(&'a mut self, k: K) -> Result<<Spec as TreeMutSpec<'a>>::CursorMut, TreeError>;

    fn get_mut<'a, K: AsRef<[u8]>>(&'a mut self, k: K) -> Result<Option<<Spec as TreeMutSpec<'a>>::GetMut>, TreeError> {
        self.entry_mut(k).map(|x| x.map(|mut y| y.get_mut()))
    }

    fn put<K: AsRef<[u8]>, V: AsRef<<Spec as MapSpec<'static>>::Value>>(&mut self, k: K, v: V) -> Result<(), TreeError>;

    fn suffix_mut<'a, K: AsRef<[u8]>>(&'a self, prefix: K) -> <Spec as TreeMutSpec<'a>>::SuffixImplMut;

    fn subrange_mut<'a, K1: AsRef<[u8]>, K2: AsRef<[u8]>>(&self, start: K1, end: K2) -> <Spec as TreeMutSpec<'a>>::SubrangeImplMut;
}

// // TODO: make these better. What's the dominant design pattern? What's the expected use case?
pub trait PersistentTreeSpec<'a>: TreeSpec<'a> {
    type TransientSpec: for<'b> TransientTreeSpec<'b>;
    type TransientImpl: TransientTree<Self::TransientSpec>;
}

pub trait PersistentTree<Spec: for<'a> PersistentTreeSpec<'a> + ?Sized>: Tree<Spec> {
    fn transient(&self) -> <Spec as PersistentTreeSpec<'static>>::TransientImpl;
}

pub trait TransientTreeSpec<'a>: TreeMutSpec<'a> {
    type PersistentSpec: for<'b> PersistentTreeSpec<'b>;
    type PersistentImpl: PersistentTree<Self::PersistentSpec>;
}

pub trait TransientTree<Spec: for<'a> TransientTreeSpec<'a> + ?Sized>: TreeMut<Spec> {
    fn persistent(&self) -> <Spec as TransientTreeSpec<'static>>::PersistentImpl;
}

pub trait HistoryTreeSpec<'a>: TreeSpec<'a> {
    type DiffSpec: for<'b> HistoryTreeSpec<'b>;
    type DiffImpl: HistoryTree<Self::DiffSpec>;
}

pub trait HistoryTree<Spec: for<'a> HistoryTreeSpec<'a> + ?Sized>: Tree<Spec> {
    fn counter(&self) -> Counter;

    fn diff<'a>(&self, c: Counter) -> <Spec as HistoryTreeSpec<'static>>::DiffImpl;
}
