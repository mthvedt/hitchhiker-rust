use std::error::Error;

use futures::Future;

use thunderhead_store::{StringSource, TdError, alloc};

use super::value::NativeValue;

pub trait EngineSpec: Sized {
    type ActiveContext: ActiveContext<Self>;
    type Context: Context<Self>;
    type Engine: Engine<Self>;
    /// An engine factory. Typically you only want one.
    type Factory: Factory<Self>;
    type FactoryHandle: FactoryHandle<Self> + Send;
    type Value: Value<Self>;
}

/// 'Activating' a Context may incur a context-switch penalty, so we want to activate
/// and deactivate these only when needed.
///
/// There may be only one ActiveContext per Context at a given time. We would love for Rust
/// to enforce this with lifetimes, but you cannot pair universal lifetimes with associated types.
pub trait ActiveContext<E: EngineSpec<ActiveContext = Self>>: Sized {
    fn eval_script(&mut self, name: &str, source: &[u8]) -> Result<E::Value, TdError>;

    fn eval_fn(&mut self, f: &mut E::Value, v: &[u8]) -> Result<E::Value, TdError>;
}

pub trait Context<E: EngineSpec<Context = Self>>: Sized {
    fn exec<R, F: FnOnce(&mut E::ActiveContext) -> R>(&mut self, f: F) -> R;
}

/// Factories are not thread-safe; they must live on a single thread.
/// Factory handles can be safely passed between threads.
/// TODO: can we remove this restriction? Then we can get rid of Handles.
pub trait Factory<E: EngineSpec<Factory = Self>>: Sized {
    fn new() -> Result<Self, TdError>;
    fn handle(&self) -> E::FactoryHandle;
}

pub trait FactoryHandle<E: EngineSpec<FactoryHandle = Self>>: Send + Sized {
    fn new_engine(&mut self) -> Result<E::Engine, String>;
}

// TODO maybe an EngineSpec

/// Engines are not thread-safe; they must live on a single thread.
pub trait Engine<E: EngineSpec<Engine = Self>>: Sized {
    fn new_context(&mut self) -> Result<E::Context, TdError>;
}

pub trait Value<E: EngineSpec<Value = Self>>: Sized {
    // fn from_native_value(v: NativeValue) -> Self;

    // TODO: a real error type
    fn to_native_value(&mut self, &mut E::ActiveContext) -> Result<NativeValue, TdError>;

    // TODO: a better story for errors
    fn debug_string(&mut self, &mut E::ActiveContext) -> Result<String, TdError>;

    fn serialize(&mut self, &mut E::ActiveContext) -> Result<String, TdError>;

    fn is_function(&self) -> bool;
}
