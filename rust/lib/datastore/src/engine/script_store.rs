use std::collections::HashMap;

use thunderhead_store::TdError;
use thunderhead_store::alloc::{self, Scoped};

use super::ScriptStore;

#[derive(Clone)]
pub struct ComboScriptStore<S1, S2> {
    s1: S1,
    s2: S2,
}

impl<S1, S2> ComboScriptStore<S1, S2> {
    pub fn new(first: S1, second: S2) -> Self {
        ComboScriptStore {
            s1: first,
            s2: second,
        }
    }
}

impl<S1, S2> ScriptStore for ComboScriptStore<S1, S2> where
S1: ScriptStore,
S2: ScriptStore,
{
    fn load(&self, s: &[u8]) -> Result<Option<Box<Scoped<[u8]>>>, TdError> {
        self.s1.load(s).and_then(|r_opt| match r_opt {
            Some(r) => Ok(Some(r)),
            None => self.s2.load(s)
        })
    }
}

#[derive(Clone)]
pub struct StaticMapScriptStore {
    inner: HashMap<&'static [u8], &'static [u8]>,
}

impl StaticMapScriptStore {
    pub fn new(inner: HashMap<&'static [u8], &'static [u8]>) -> Self {
        StaticMapScriptStore {
            inner: inner,
        }
    }
}

impl ScriptStore for StaticMapScriptStore {
    fn load(&self, s: &[u8]) -> Result<Option<Box<Scoped<[u8]>>>, TdError> {
        let bytes_opt = self.inner.get(s).map(|script| {
            let r: Box<alloc::Scoped<[u8]>> = Box::new(alloc::ScopedRef(*script));
            r
        });
        Ok(bytes_opt)
    }
}
