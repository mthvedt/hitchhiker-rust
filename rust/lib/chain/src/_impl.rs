//! Types that must be public for interface reasons,
//! but that we don't want to expose.

use std::marker::PhantomData;

/// An instance of PhantomData that implements Send.
pub struct PhantomDataSend<T>(PhantomData<T>);

unsafe impl<T> Send for PhantomDataSend<T> {}

/// An instance of PhantomData that implements Send + Sync.
pub struct PhantomDataSync<T>(PhantomData<T>);

unsafe impl<T> Send for PhantomDataSync<T> {}
unsafe impl<T> Sync for PhantomDataSync<T> {}

/// Marker used to note when data is send.
pub struct SendMarker;

/// Marker used to note when data is Sync.
pub struct SyncMarker;
