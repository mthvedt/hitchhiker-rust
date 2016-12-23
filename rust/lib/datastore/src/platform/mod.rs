// // use futures::Future;
// //
// // use thunderhead_store::{StringSource, KvSink, TdError};
// // use thunderhead_store::tdfuture::{BoxFuture, FutureExt};
// //
// // use engine::{Processor, RuntimeHandle};
// // use lens::{MapStore, StringLens};
// //
// // /// A Rest HTTP view, with web admin functionality, wrapping a Thunderhead Rest store.
// // pub struct SchemaStore<S, D> {
// //     src_repo: S,
// //     data_repo: D,
// //     // TODO: multithreading.
// //     runtime: RuntimeHandle,
// //     master: Processor,
// // }
// //
// // impl<S: StringSource + Clone + 'static, D: KvSink + 'static> SchemaStore<S, D> {
// //     fn bootstrap(s: S, d: D) -> BoxFuture<Self, TdError> {
// //         let mut r = RuntimeHandle::new_runtime();
// //
// //         // TODO what now? Need to read a source from fs, git, test dir, &c.
// //         /*
// //         1. Load core library
// //         2. open datastore graph
// //         */
// //         Processor::from_source(r.new_environment(), "schema.js", s.clone())
// //         .map(|pxr| SchemaStore {
// //             src_repo: s,
// //             data_repo: d,
// //             runtime: r,
// //             master: pxr,
// //         }).td_boxed()
// //     }
// // }
//
// // #[derive(Eq, Hash, PartialEq)]
// // pub enum EngineType {
// //     JS,
// // }
// //
// // pub struct EngineCoordinator {
// //     engines: Map<EngineType,
// // }
// //
// // pub struct EngineThread {
// //
// // }
//
// //! The glue code for plugging the front-end into the backend.
// //!
// //! This module contains bootstrappers, which load engines and provide them
// //! with standard libraries. Bootstrappers also provide the context
// //! in which schemata are evaluated.
//
// use engine::{ActiveContext, Context, Engine, Factory, FactoryHandle};
// use engine::spidermonkey::Spec as JsSpec;
// use system::SystemScriptStore;
//
// /// Bootstrappers load engines and provide them with standard libraries.
// /// They are Thunderhead's standard interface to an engine.
// /// If the language supports schemata, this trait provides that also.
// ///
// /// Right now, there is only one bootstrapper, the JS bootstrapper.
// ///
// /// TODO: maybe merge this with Engine.
// pub trait Bootstrapper {
//
// }
//
// // TODO: make an executor module.
// /// Loads the given master schema and starts the executor.
// /// Right now we only support JS. We might support more engines in the future.
//
// /// Loads the JS library context, used by the JS schema bootstrapper.
// pub fn js_library_context() {
//     let f = Spec::new_factory(SystemScriptStore).unwrap();
//
//     let mut master_context = f.handle().new_engine().unwrap().new_context().unwrap();
//
//     master_context.exec(|acx| {
//         acx
//         .eval_file("js/td.js")
//         .and_then(|_| acx.eval_file("js/syslib.js"))
//     }).unwrap();
//
//     // So what do we need to do?
//     // * Bootstrap from system source
//     // * Use master context to create master object
//     // * Feed master object to schema generator
//     // * Interpret schema
//
//     // Q: How do we blend contexts?
//     // Anything the master context gives us
//     // may refer to that whole environment.
//     //
//     // Soooo, perhaps we shouldn't.
//     // We can, however, 'submodule-ify' contexts.
// }
//
// pub fn js_load_schema(cx: JsSpec::Context, &mut schema_source_name: &[u8]) {
//     cx.exec(|acx| {
//
//     })
// }
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn smoke_test() {
//         bootstrap("".as_ref());
//     }
// }
