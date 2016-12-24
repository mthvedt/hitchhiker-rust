use std::borrow::Borrow;
use std::collections::HashMap;

use futures::future;

// For god-knows-what reason, importing Scoped breaks lazy_static.
use thunderhead_store::alloc;
use thunderhead_store::{Range, Source, TdError};

use engine::ScriptStore;
use engine::script_store::StaticMapScriptStore;

// TODO: this is not thread safe. or is it?
// TODO: macro helpers.
lazy_static! {
    // TODO: Box<[u8]> instead. or even &'static
    static ref SYSTEM_SCRIPT_MAP: HashMap<&'static [u8], &'static [u8]> = {
        let mut m = HashMap::new();

        // TODO: a macro
        m.insert("td/Td.js".as_ref(), include_str!("js/td/Td.js").as_ref());

        m
    };

    pub static ref SYSTEM_SCRIPT_STORE: StaticMapScriptStore = StaticMapScriptStore::new(SYSTEM_SCRIPT_MAP.clone());
}

// pub struct SystemScripts;
//
// pub struct StrWrapper {
//     inner: &'static str,
// }
//
// impl Borrow<[u8]> for StrWrapper {
//     fn borrow(&self) -> &[u8] {
//         self.inner.as_ref()
//     }
// }
//
// impl Source<[u8]> for SystemScripts {
// 	type Get = StrWrapper;
//     type GetF = future::Ok<Option<Self::Get>, TdError>;
//
//     fn get<K: alloc::Scoped<[u8]>>(&mut self, k: K) -> Self::GetF {
//         future::ok(SYSTEM_SCRIPTS.get(k.get().unwrap()).cloned().map(|s| StrWrapper {
//             inner: s,
//         }))
//     }
//
//     #[allow(unused_variables)]
//     fn subtree<K: alloc::Scoped<[u8]>>(&mut self, k: K) -> Self {
//         panic!("not implemented")
//     }
//
//     #[allow(unused_variables)]
//     fn subrange(&mut self, range: Range) -> Self {
//         panic!("not implemented")
//     }
// }
