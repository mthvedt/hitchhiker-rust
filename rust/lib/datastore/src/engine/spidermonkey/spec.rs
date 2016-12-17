use thunderhead_store::TdError;
use thunderhead_store::alloc::Scoped;

use engine::traits;

use super::factory;

pub struct Spec;

// TODO: for now, Stores may block. This is because:
// 1) includes should be rare, and almost never long-blocking
// 2) for simplicity, user code will generally load/require/include inline
// 3) doing this with futures requires messy generic allocs
pub trait ScriptStore: Send + Sync + 'static {
    // TODO: scoped is kind of messy here. Is there a better option?
    fn load(&self, s: &str) -> Option<Box<Scoped<[u8]>>>;
}

impl Spec {
    pub fn new_factory<S: ScriptStore>(s: S) -> Result<factory::Factory, TdError> {
        factory::new_factory(s)
    }
}

impl traits::EngineSpec for Spec {
    type ActiveContext = super::active_context::ActiveContext;
    type Context = super::context::Context;
    type Engine = super::engine::Engine;
    type Factory = super::factory::Factory;
    type FactoryHandle = super::factory::FactoryHandle;
    type Value = super::value::RootedVal;
}
