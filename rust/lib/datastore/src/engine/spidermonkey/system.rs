use std::borrow::Borrow;
use std::collections::BTreeMap;

use futures::future;

// For god-knows-what reason, importing Scoped breaks lazy_static.
use thunderhead_store::alloc;
use thunderhead_store::{Range, Source, TdError};

use engine::ScriptStore;

// TODO: this is not thread safe. or is it?
// TODO: macro helpers.
lazy_static! {
    // TODO: Box<[u8]> instead. or even &'static
    static ref SYSTEM_SCRIPTS: BTreeMap<&'static [u8], &'static str> = {
        let mut m = BTreeMap::new();

        // TODO: a macro
        m.insert("js/Td.js".as_ref(), include_str!("js/Td.js"));
        m.insert("js/serialize_json.js".as_ref(), include_str!("js/serialize_json.js"));

        m
    };
}

pub struct SystemScriptStore;

impl ScriptStore for SystemScriptStore {
    fn load(&self, s: &str) -> Result<Option<Box<alloc::Scoped<[u8]>>>, TdError> {
        let sb: &[u8] = s.as_ref();
        let r = SYSTEM_SCRIPTS.get(sb).map(|script| {
            let r2: Box<alloc::Scoped<[u8]>> = Box::new(alloc::ScopedRef(script.as_ref()));
            r2
        });
        Ok(r)
    }
}

pub struct SystemScripts;

pub struct StrWrapper {
    inner: &'static str,
}

impl Borrow<[u8]> for StrWrapper {
    fn borrow(&self) -> &[u8] {
        self.inner.as_ref()
    }
}

impl Source<[u8]> for SystemScripts {
	type Get = StrWrapper;
    type GetF = future::Ok<Option<Self::Get>, TdError>;

    fn get<K: alloc::Scoped<[u8]>>(&mut self, k: K) -> Self::GetF {
        future::ok(SYSTEM_SCRIPTS.get(k.get().unwrap()).cloned().map(|s| StrWrapper {
            inner: s,
        }))
    }

    #[allow(unused_variables)]
    fn subtree<K: alloc::Scoped<[u8]>>(&mut self, k: K) -> Self {
        panic!("not implemented")
    }

    #[allow(unused_variables)]
    fn subrange(&mut self, range: Range) -> Self {
        panic!("not implemented")
    }
}
