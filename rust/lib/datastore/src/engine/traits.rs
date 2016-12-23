use thunderhead_store::TdError;
use thunderhead_store::alloc::Scoped;

use super::value::NativeValue;

/// An engine for use with Thunderhead.
pub trait EngineSpec: Sized {
    type ActiveContext: ActiveContext<Self>;
    type Context: Context<Self>;
    type Engine: Engine<Self>;
    // TODO: Factories should have a standard constructor.
    type Factory: Factory<Self>;
    type FactoryHandle: FactoryHandle<Self> + Send;
    type Value: Value<Self>;
}

/// An active execution context.
///
/// 'Activating' a Context may incur a context-switch penalty, so we want to activate
/// and deactivate these only when needed.
///
/// There may be only one ActiveContext per Context at a given time. We would love for Rust
/// to enforce this with lifetimes, but you cannot pair universal lifetimes with associated types.
pub trait ActiveContext<E: EngineSpec<ActiveContext = Self>>: Sized {
}

/// An execution contet.
///
/// Execution contexts are 'inactive', and must be activated to be used.
/// Activation may incur some kind of context-switch penalty.
pub trait Context<E: EngineSpec<Context = Self>>: Sized {
    fn exec<R, F: FnOnce(&mut E::ActiveContext) -> R>(&mut self, f: F) -> R;
}

/// An Engine. An Engine can produce Contexts to execute code.
/// Engines are not thread-safe; they must live on a single thread.
pub trait Engine<E: EngineSpec<Engine = Self>>: Sized {
    fn new_context(&mut self, root_script: &[u8]) -> Result<E::Context, TdError>;
}

/// A source of Engines. Ideally, you want one Factory per EngineSpec per process.
///
/// Factories are not thread-safe; they must live on a single thread.
/// Instead, send FactoryHandles to different threads.
/// It is an error to destroy a Factory while handles are extant.
pub trait Factory<E: EngineSpec<Factory = Self>>: Sized {
    fn handle(&self) -> E::FactoryHandle;
}

/// A handle to a Factory.
/// It is an error to keep a Handle around to a destroyed Factory.
pub trait FactoryHandle<E: EngineSpec<FactoryHandle = Self>>: Send + Sized {
    fn new_engine(&mut self) -> Result<E::Engine, String>;
}

// TODO: for now, Stores may block. This is because:
// 1) includes should be rare, and almost never long-blocking
// 2) for simplicity, user code will generally load/require/include inline
// 3) doing this with futures requires messy generic allocs
pub trait ScriptStore: Send + Sync + 'static {
    // TODO: scoped is kind of messy here. Is there a better option?
    fn load(&self, s: &str) -> Result<Option<Box<Scoped<[u8]>>>, TdError>;
}

// TODO: maybe get rid of Value
pub trait Value<E: EngineSpec<Value = Self>>: Sized {
    // fn from_native_value(v: NativeValue) -> Self;

    // TODO: a real error type
    fn to_native_value(&mut self, &mut E::ActiveContext) -> Result<NativeValue, TdError>;

    // TODO: a better story for errors
    fn debug_string(&mut self, &mut E::ActiveContext) -> Result<String, TdError>;

    fn serialize(&mut self, &mut E::ActiveContext) -> Result<String, TdError>;

    // TODO: this probably could use a refactor.
    fn is_function(&self, &mut E::ActiveContext) -> bool;
}
