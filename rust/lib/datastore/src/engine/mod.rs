//! Thunderhead-specific wrappers for dynamic languages that support a "nested context" facility.
//! Right now only JS is supported, and this package is pretty JS-specific,
//! unfortunately.
//!
//! An engine is a heavyweight language environment that can produce "contexts",
//! isolated subregions that can execute code.
//! There are two kinds of contexts: schemata and executors.
//! A schema is an object, the details of which are hidden, that can be passed to an executor.
//! An executor can be passed in a schema to be executed. Executors are themselves schemata.
//! In general, schemata are user code, and executors are system code.
//!
//! Engines are not isolated things. They are Thunderhead-specific.
//! In general, they know about Thunderhead KV stores and require a Thunderhead environment
//! to be set up around them.
//!
//! Right now we only have a JS engine, and it's not clear how this abstraction
//! should change to accomadate other engines. Abstraction is good qua abstraction, though.

/*
- eval scripts in a context
- plug context into executor
*/

mod error;
pub use self::error::*;

mod processor;
pub use self::processor::ProcessorHandle;

pub mod spidermonkey;

mod traits;
pub use self::traits::*;

mod value;
pub use self::value::NativeValue;
