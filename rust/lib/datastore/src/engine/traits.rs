use std::error::Error;

use futures::Future;

use thunderhead_store::{StringSource, TdError, alloc};

use super::value::NativeValue;

// TODO: work out associated types. Should everything depend on Engine?

/// 'Activating' a Context may incur a context-switch penalty, so we want to activate
/// and deactivate these only when needed.
///
/// There may be only one ActiveContext per Context at a given time. We would love for Rust
/// to enforce this with lifetimes, but you cannot pair universal lifetimes with associated types.
pub trait ActiveContext: Sized {
    type Engine: Engine<ActiveContext = Self>;

    fn eval_script(&mut self, name: &str, source: &[u8]) -> Result<<Self::Engine as Engine>::Value, TdError>;

    fn eval_fn(&mut self, f: &mut <Self::Engine as Engine>::Value, v: &[u8]) -> Result<<Self::Engine as Engine>::Value, TdError>;
}

pub trait Context: Sized {
    type Engine: Engine<Context = Self>;

    fn exec<R, F: FnOnce(&mut <Self::Engine as Engine>::ActiveContext) -> R>(&mut self, f: F) -> R;
}

/// Factories are not thread-safe; they must live on a single thread.
/// Factory handles can be safely passed between threads.
/// TODO: can we remove this restriction? Then we can get rid of Handles.
pub trait Factory: Sized {
    type Engine: Engine<Factory = Self>;

    fn new() -> Result<Self, TdError>;
    fn handle(&self) -> <Self::Engine as Engine>::FactoryHandle;
}

pub trait FactoryHandle: Sized + Send + Sync {
    type Engine: Engine<FactoryHandle = Self>;

    fn new_engine(&mut self) -> Result<Self::Engine, String>;
}

// TODO maybe an EngineSpec

/// Engines are not thread-safe; they must live on a single thread.
pub trait Engine: Sized {
    type ActiveContext: ActiveContext<Engine = Self>;
    type Context: Context<Engine = Self>;
    /// An engine factory. Typically you only want one.
    type Factory: Factory<Engine = Self>;
    type FactoryHandle: FactoryHandle<Engine = Self>;
    type Value: Value<Engine = Self>;

    fn new_context(&mut self) -> Result<Self::Context, TdError>;
}

pub trait Value: Sized {
    type Engine: Engine;
    // fn from_native_value(v: NativeValue) -> Self;

    // TODO: a real error type
    fn to_native_value(&mut self, &mut <Self::Engine as Engine>::ActiveContext) -> Result<NativeValue, TdError>;

    // TODO: a better story for errors
    fn debug_string(&mut self, &mut <Self::Engine as Engine>::ActiveContext) -> Result<String, TdError>;

    fn serialize(&mut self, &mut <Self::Engine as Engine>::ActiveContext) -> Result<String, TdError>;

    fn is_function(&self) -> bool;
}
