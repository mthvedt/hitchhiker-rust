use std::collections::HashMap;

use thunderhead_store::TdError;
use thunderhead_store::alloc::Scoped;

use engine::{ScriptStore, script_store};

use super::factory;
use super::system;

pub struct EmptyScriptStore;

impl ScriptStore for EmptyScriptStore {
    fn load(&self, _: &[u8]) -> Result<Option<Box<Scoped<[u8]>>>, TdError> {
        Ok(Some(Box::new("".as_ref())))
    }
}

// TODO maybe should be a test function...
pub fn empty_store_factory() -> Result<factory::Factory, TdError> {
    factory::new_factory(EmptyScriptStore)
}

pub fn test_store_factory(sources: HashMap<&'static [u8], &'static [u8]>) -> Result<factory::Factory, TdError> {
    factory::new_factory(script_store::ComboScriptStore::new(
        script_store::StaticMapScriptStore::new(sources),
        system::SYSTEM_SCRIPT_STORE.clone()))
}
