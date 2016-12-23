use thunderhead_store::TdError;
use thunderhead_store::alloc::Scoped;

use engine::ScriptStore;

use super::factory;

pub struct EmptyScriptStore;

impl ScriptStore for EmptyScriptStore {
    fn load(&self, s: &str) -> Result<Option<Box<Scoped<[u8]>>>, TdError> {
        Ok(None)
    }
}

// TODO maybe should be a test function...
pub fn new_factory() -> Result<factory::Factory, TdError> {
    factory::new_factory(EmptyScriptStore)
}
