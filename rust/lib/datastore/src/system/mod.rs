use std::collections::BTreeMap;

use futures::future;

// For god-knows-what reason, importing Scoped breaks lazy_static.
use thunderhead_store::alloc;
use thunderhead_store::{Range, Source, StringSource, TdError};

// // TODO: move BTreeSource to store.
// struct BTreeSource<K, V> {
//     inner: BTreeMap<K, V>,
// }
//
// impl<V: 'static + Clone> Source<V> for BTreeSource<Box<[u8]>, V> {
//     type Get = V;
//     type GetF = future::Ok<Option<V>, TdError>;
//
//     fn get<K: alloc::Scoped<[u8]>>(&mut self, k: K) -> Self::GetF {
//         // TODO: don't unwrap
//         future::ok(self.inner.get(k.get().unwrap()).cloned())
//     }
//
//     fn subtree<K: alloc::Scoped<[u8]>>(&mut self, k: K) -> Self {
//         panic!("not implemented")
//     }
//
//     fn subrange(&mut self, range: Range) -> Self {
//         panic!("not implemented")
//     }
// }

// TODO: this is not thread safe.
// TODO: macro helpers.
lazy_static! {
    // TODO: Box<[u8]> instead. or even &'static
    static ref SYSTEM_SCRIPTS: BTreeMap<&'static [u8], &'static str> = {
        let mut m = BTreeMap::new();

        // TODO: a macro
        m.insert("js/parse_json".as_ref(), include_str!("js/parse_json.js"));

        m
    };
}

pub struct SystemScripts;

impl Source<str> for SystemScripts {
	type Get = &'static str;
    type GetF = future::Ok<Option<Self::Get>, TdError>;

    fn get<K: alloc::Scoped<[u8]>>(&mut self, k: K) -> Self::GetF {
        future::ok(SYSTEM_SCRIPTS.get(k.get().unwrap()).cloned())
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

impl StringSource for SystemScripts {}
