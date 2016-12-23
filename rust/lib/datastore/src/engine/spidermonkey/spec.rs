use thunderhead_store::TdError;

use engine::traits;

use super::factory;

pub struct Spec;

impl Spec {
    pub fn new_factory<S: traits::ScriptStore>(s: S) -> Result<factory::Factory, TdError> {
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
