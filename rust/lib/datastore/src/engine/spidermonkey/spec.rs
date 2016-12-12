use engine::traits;

pub struct Spec;

impl traits::EngineSpec for Spec {
    type ActiveContext = super::active_context::ActiveContext;
    type Context = super::context::Context;
    type Engine = super::engine::Engine;
    type Factory = super::factory::Factory;
    type FactoryHandle = super::factory::FactoryHandle;
    type Value = super::value::RootedVal;
}
