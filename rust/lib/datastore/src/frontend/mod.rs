// use futures::Future;
//
// use thunderhead_store::{StringSource, KvSink, TdError};
// use thunderhead_store::tdfuture::{BoxFuture, FutureExt};
//
// use engine::{Processor, RuntimeHandle};
// use lens::{MapStore, StringLens};
//
// /// A Rest HTTP view, with web admin functionality, wrapping a Thunderhead Rest store.
// pub struct SchemaStore<S, D> {
//     src_repo: S,
//     data_repo: D,
//     // TODO: multithreading.
//     runtime: RuntimeHandle,
//     master: Processor,
// }
//
// impl<S: StringSource + Clone + 'static, D: KvSink + 'static> SchemaStore<S, D> {
//     fn bootstrap(s: S, d: D) -> BoxFuture<Self, TdError> {
//         let mut r = RuntimeHandle::new_runtime();
//
//         // TODO what now? Need to read a source from fs, git, test dir, &c.
//         /*
//         1. Load core library
//         2. open datastore graph
//         */
//         Processor::from_source(r.new_environment(), "schema.js", s.clone())
//         .map(|pxr| SchemaStore {
//             src_repo: s,
//             data_repo: d,
//             runtime: r,
//             master: pxr,
//         }).td_boxed()
//     }
// }
