use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use js::jsapi;

use thunderhead_store::TdError;

use engine::traits;

use super::engine;
use super::spec::Spec;

struct _JSInit(Result<(), &'static str>);

// Unfortunately, Mozilla js doesn't allow initting/destroying JS multiple times in one process.
// So we must init and destroy it as a global. JS_Shutdown cleans up resources on some platforms.
lazy_static! {
    static ref _JS_INIT: _JSInit = {
        unsafe {
            if jsapi::JS_Init() {
                _JSInit(Ok(()))
            } else {
                _JSInit(Err("FATAL: Could not init JS"))
            }
        }
    };
}

impl Drop for _JSInit {
    fn drop(&mut self) {
        unsafe {
            jsapi::JS_ShutDown();
        }
    }
}

pub struct FactoryInner {
    num_handles: AtomicU64,
}

pub struct Factory {
    // Ideally Factory should have a master JSRuntime.
    // However, this seems to break multithreading in undocumented ways.
    // We retain safety-checking for Factory because we want to make sure we use code patterns
    // that support other kinds of Factories in the future.
    // But this class doesn't do anything except safety check.
    inner: Arc<FactoryInner>,
}

impl traits::Factory<Spec> for Factory {
    fn new() -> Result<Self, TdError> {
        let inner = FactoryInner {
            num_handles: AtomicU64::new(0),
        };

        let r = Factory {
            inner: Arc::new(inner),
        };

        Ok(r)
    }

    fn handle(&self) -> FactoryHandle {
        self.inner.num_handles.fetch_add(1, Ordering::SeqCst);

        FactoryHandle {
            inner: self.inner.clone(),
        }
    }
}

impl Drop for Factory {
    fn drop(&mut self) {
        if self.inner.num_handles.load(Ordering::SeqCst) != 0 {
            // TODO: This will terminate the program. It would be nice to have
            // something that doesn't terminate.
            panic!("FATAL: Dropping factory while handles are extant");
        }
    }
}

pub struct FactoryHandle {
    inner: Arc<FactoryInner>,
}

impl Drop for FactoryHandle {
    fn drop(&mut self) {
        self.inner.num_handles.fetch_sub(1, Ordering::SeqCst);
    }
}

impl traits::FactoryHandle<Spec> for FactoryHandle {
    fn new_engine(&mut self) -> Result<engine::Engine, String> {
        _JS_INIT.0.map_err(|s| panic!(s)).unwrap();

        engine::new_engine()
    }
}
