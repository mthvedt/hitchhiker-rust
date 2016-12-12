use std;
use std::cell::{RefCell};
use std::marker::PhantomData;
use std::ptr::{self, Unique};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};

use js::jsapi::{self, JSRuntime};

use thunderhead_store::TdError;

use engine::error::{ErrorReporter, LoggingErrorReporter};
use engine::traits::{self, Engine};

use super::engine::{self, EngineInner};
use super::globals::{self, ActiveGlobals};
use super::spec::Spec;

// TODO: what do these do?
const DEFAULT_HEAP_SIZE: u32 = 32 * 1024 * 1024;
const DEFAULT_CHUNK_SIZE: u32 = 20 * 1024 * 1024;

/// Stack size, borrowed from Gecko.
const STACK_SIZE: usize = 1024 * 1024;

/// Borrowed from Gecko. The goal is to execute 10 or more extra stack frames (stack frames are very large)
/// for trusted system calls.
const SYSTEM_CODE_STACK_BUFFER: usize = 32 * 1024;

// /// Borrowed from Gecko. Gives us stack space for trusted script calls.
const TRUSTED_SCRIPT_STACK_BUFFER: usize = 128 * 1024;

struct RuntimeSync(*mut JSRuntime);
unsafe impl Send for RuntimeSync {}
unsafe impl Sync for RuntimeSync {}

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
        unsafe {
            _JS_INIT.0.map_err(|s| panic!(s)).unwrap();
            
            let js_runtime = jsapi::JS_NewRuntime(
                DEFAULT_HEAP_SIZE,
                DEFAULT_CHUNK_SIZE,
                ptr::null_mut());
            assert!(!js_runtime.is_null(), "Could not build JS runtime");

            // This next line (and the below comments) were taken from Servo's mozjs project.
            // Unconstrain the runtime's threshold on nominal heap size, to avoid
            // triggering GC too often if operating continuously near an arbitrary
            // finite threshold. This leaves the maximum-JS_malloc-bytes threshold
            // still in effect to cause periodical, and we hope hygienic,
            // last-ditch GCs from within the GC's allocator.
            jsapi::JS_SetGCParameter(
                js_runtime, jsapi::JSGCParamKey::JSGC_MAX_BYTES, std::u32::MAX);

            let js_context = jsapi::JS_GetContext(js_runtime);
            assert!(!js_context.is_null(), "Could not get JS context");

            jsapi::SetWarningReporter(js_runtime, Some(globals::ActiveGlobals::report_warning));

            let _g = ActiveGlobals::set_scoped(js_context, LoggingErrorReporter);

            // I'm not really sure how requests work in SpiderMonkey. They seem to be
            // about preventing GC in multithreaded contexts, but threads are turned off by default?
            // Anyway, we will always be in a single thread, so no harm in keeping the request open for the lifetime
            // of the Context.
            jsapi::JS_BeginRequest(js_context);

            jsapi::JS_SetNativeStackQuota(
                js_runtime,
                STACK_SIZE,
                STACK_SIZE - SYSTEM_CODE_STACK_BUFFER,
                STACK_SIZE - SYSTEM_CODE_STACK_BUFFER - TRUSTED_SCRIPT_STACK_BUFFER);

            jsapi::InitSelfHostedCode(js_context);

            // TODO: what about these?
            // let runtimeopts = jsapi::RuntimeOptionsRef(js_runtime);
            // (*runtimeopts).set_baseline_(true);
            // (*runtimeopts).set_ion_(true);
            // (*runtimeopts).set_nativeRegExp_(true);

            Ok(engine::new_engine(&mut *js_runtime, &mut *js_context))
        }
    }
}
