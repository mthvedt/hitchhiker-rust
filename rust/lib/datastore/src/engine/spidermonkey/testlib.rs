use thunderhead_store::TdError;
use thunderhead_store::alloc::Scoped;

use super::factory;
use super::spec::ScriptStore;

pub struct EmptyScriptStore;

impl ScriptStore for EmptyScriptStore {
    fn load(&self, s: &str) -> Option<Box<Scoped<[u8]>>> {
        None
    }
}

// TODO maybe should be a test function...
pub fn new_factory() -> Result<factory::Factory, TdError> {
    factory::new_factory(EmptyScriptStore)
}
