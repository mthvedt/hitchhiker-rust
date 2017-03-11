use std::io::{self, Read, Write};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

// Sketch for sync:

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

pub struct TreeRef<'a, T: 'a> {
    _marker: PhantomData<&'a T>,
    inner: T,
}

impl<'a, T: 'a> Deref for TreeRef<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

//We introduce constraints here. We can always relax them later.
/// A handle to an immutable byte store.
pub trait Tree<V: Read>: Sized {
    fn get<K: AsRef<[u8]>>(&self, k: K) -> Result<V, TreeError>;

    // TODO: entry interface, cursor interface

    // TODO: existence check

    // TODO: must this return self?
    /// This function returns Self. Ideally, we'd like to be able to return an arbitrary type of subtree,
    /// but this makes Rust's constraint checker behave oddly in some cases, particularly with subtraits.
    fn subtree<'a, K: AsRef<[u8]>>(&'a self, k: K) -> TreeRef<'a, Self>;

    fn subrange<'a, K1: AsRef<[u8]>, K2: AsRef<[u8]>>(&self, start: K1, end: K2) -> TreeRef<'a, Self>;
}

pub struct SnapshotRef<'a, O: 'a, T: 'a> {
    _marker: PhantomData<&'a mut O>,
    inner: T,
}

impl<'a, O: 'a, T: 'a> Deref for SnapshotRef<'a, O, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

pub struct TreeRefMut<'a, T: 'a> {
    _marker: PhantomData<&'a mut T>,
    inner: T,
}

impl<'a, T: 'a> Deref for TreeRefMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<'a, T: 'a> DerefMut for TreeRefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

/// A handle to a byte store. Owning this handle implies owning the byte store;
/// only one context may mutably write to and delete from it. Any number of immutable snapshots
/// can be obtained.
pub trait TreeMut<V: Read + Write>:  Sized {
    type Snapshot: Tree<V>;

    fn get<K: AsRef<[u8]>>(&self, k: K) -> Result<V, TreeError>;

    // TODO: entry interface, cursor interface

    fn put<K: AsRef<[u8]>, VRef: AsRef<V>>(&mut self, k: K, v: VRef) -> Result<(), TreeError>;

    fn subtree<'a, K: AsRef<[u8]>>(&'a mut self, k: K) -> TreeRefMut<'a, Self>;

    fn subrange<'a, K1, K2>(&'a mut self, start: K1, end: K2) -> TreeRefMut<'a, Self> where
    K1: AsRef<[u8]>, K2: AsRef<[u8]>;
}
