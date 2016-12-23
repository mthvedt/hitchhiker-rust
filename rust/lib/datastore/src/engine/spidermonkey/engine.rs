use std;
use std::cell::RefCell;
use std::ptr::{self, Unique};
use std::rc::Rc;

use js::jsapi::{self, JSContext, JSRuntime};

use thunderhead_store::TdError;

use engine::error::LoggingErrorReporter;
use engine::traits;

use super::{context, executor};
use super::factory::FactoryHandle;
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

pub struct EngineInner {
    inner: Unique<JSContext>,
    inner_runtime: Unique<JSRuntime>,
    factory: FactoryHandle,
}

impl Drop for EngineInner {
    fn drop(&mut self) {
        unsafe {
            jsapi::JS_EndRequest(self.inner.get_mut());
            jsapi::JS_DestroyRuntime(self.inner_runtime.get_mut());
        }
    }
}

pub struct Engine {
    /// We basically use this RefCell as a checked UnsafeCell.
    /// The invariant must hold that any borrow/mut borrow of this must be in the scope of
    /// a borrow/mut borrow of Engine.
    inner: Rc<RefCell<EngineInner>>,
}

pub fn new_engine(handle: FactoryHandle) -> Result<Engine, String> {
    unsafe {
        let js_runtime = jsapi::JS_NewRuntime(
            DEFAULT_HEAP_SIZE,
            DEFAULT_CHUNK_SIZE,
            // Spidermonkey requires one engine per thread (contrary to the docs recommending one engine per process.)
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

        let inner = EngineInner {
            inner_runtime: Unique::new(js_runtime),
            inner: Unique::new(js_context),
            factory: handle,
        };

        Ok(Engine {
            inner: Rc::new(RefCell::new(inner)),
        })
    }
}

// so that clone isn't public
pub fn clone_engine(e: &Engine) -> Engine {
    Engine {
        inner: e.inner.clone(),
    }
}

pub fn js_context(e: &mut Engine) -> &mut JSContext {
    unsafe {
        let mut innerref = e.inner.borrow_mut();
        let p: *mut JSContext = innerref.inner.get_mut();
        &mut *p
    }
}

pub fn exec_for_factory_handle<R, F: FnOnce(&mut FactoryHandle) -> R>(e: &mut Engine, f: F) -> R {
    (f)(&mut e.inner.borrow_mut().factory)
}

impl traits::Engine<Spec> for Engine {
    fn new_context(&mut self) -> Result<context::Context, TdError> {
        Ok(context::new_context(self))
    }

    fn new_executor(&mut self) -> Result<executor::Executor, TdError> {
        Ok(executor::new_executor(self))
    }
}
