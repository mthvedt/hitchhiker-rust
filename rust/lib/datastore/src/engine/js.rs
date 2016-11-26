use std;
use std::cell::RefCell;
use std::ffi::CString;
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::ptr;
use std::ptr::Unique;
use std::rc::Rc;

use libc::{c_uint, size_t};

use js::jsapi;
use js::jsapi::{JSAutoCompartment, JSContext, JSRuntime, JSObject, Value};
use js::jsval;
use js::rust;

// TODO: name. Make this not dependent on SpiderMonkey.
pub struct Rooted<T> {
    inner: jsapi::Rooted<T>,
}

pub type RootedObject = Rooted<*mut JSObject>;
pub type RootedVal = Rooted<Value>;

impl<T> Rooted<T> {
    fn new(t: T, cx: &mut JSContext) -> Self where T: rust::RootKind {
        let mut inner = jsapi::Rooted::new_unrooted(t);
        unsafe {
            inner.add_to_root_stack(cx);
        }

        Rooted {
            inner: inner,
        }
    }

    fn handle<'a>(&'a self) -> Handle<'a, T> {
        Handle {
            inner: unsafe { jsapi::Handle::from_marked_location(&self.inner.ptr) },
            _p: PhantomData,
        }
    }

    fn handle_mut<'a>(&'a mut self) -> HandleMut<'a, T> {
        HandleMut {
            inner: unsafe { jsapi::MutableHandle::from_marked_location(&mut self.inner.ptr) },
            _p: PhantomData,
        }
    }
}

struct Handle<'a, T: 'a> {
    inner: jsapi::Handle<T>,
    _p: PhantomData<&'a T>,
}

struct HandleMut<'a, T: 'a> {
    inner: jsapi::MutableHandle<T>,
    _p: PhantomData<&'a T>,
}

type HandleMutVal<'a> = HandleMut<'a, Value>;

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

/// The master runtime for all JS runtimes in Thunderhead.
fn master_runtime() -> *mut JSRuntime {
    struct MasterRuntime(*mut JSRuntime);
    unsafe impl Sync for MasterRuntime {}

    lazy_static! {
        static ref INNER: MasterRuntime = {
            unsafe {
                assert!(jsapi::JS_Init(), "FATAL: Could not init JS");
                let runtime = jsapi::JS_NewRuntime(
                    DEFAULT_HEAP_SIZE,
                    DEFAULT_CHUNK_SIZE,
                    ptr::null_mut() // Parent runtime--none
                );
                assert!(!runtime.is_null(), "FATAL: Could not allocate master JS runtime");

                let context = jsapi::JS_GetContext(runtime);
                jsapi::JS_BeginRequest(context);
                assert!(!context.is_null(), "FATAL: Could not get master JS context");
                jsapi::InitSelfHostedCode(context);
                jsapi::JS_EndRequest(context);

                MasterRuntime(runtime)
            }
        };
    }

    INNER.0
}

/// A fully-initialized Javascript runtime.
///
/// In this impl, it is actually a Spidermonkey JSRuntime + JSContext.
pub struct Runtime {
    inner: Unique<JSContext>,
    inner_runtime: Unique<JSRuntime>,
}

impl Runtime {
    pub fn new() -> Self {
        unsafe {
            let js_runtime = jsapi::JS_NewRuntime(DEFAULT_HEAP_SIZE, DEFAULT_CHUNK_SIZE, master_runtime());
            assert!(!js_runtime.is_null(), "Out of memory allocating JS runtime");

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

            // TODO
            // jsapi::SetWarningReporter(js_runtime, Some(report_warning));

            Runtime {
                inner_runtime: Unique::new(js_runtime),
                inner: Unique::new(js_context),
            }
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        unsafe {
            jsapi::JS_EndRequest(self.inner.get_mut());
            jsapi::JS_DestroyRuntime(self.inner_runtime.get_mut());
        }
    }
}

#[derive(Clone)]
pub struct RuntimeHandle {
    inner: Rc<RefCell<Runtime>>,
}

impl RuntimeHandle {
    fn new() -> Self {
        RuntimeHandle {
            inner: Rc::new(RefCell::new(Runtime::new())),
        }
    }

    fn context(&mut self) -> &mut JSContext {
        unsafe {
            let mut b = self.inner.borrow_mut();
            let p: *mut JSContext = b.inner.get_mut();
            &mut *p
        }
    }

    pub fn new_environment(&mut self) -> Environment {
        Environment::new(self)
    }
}

pub struct Environment {
    parent: RuntimeHandle,
    global: RootedObject,
}

impl Environment {
    fn new(parent: &mut RuntimeHandle) -> Self {
        unsafe {
            let g = jsapi::JS_NewGlobalObject(parent.context(),
                &rust::SIMPLE_GLOBAL_CLASS, // Default global class. TODO: investigate.
                ptr::null_mut(), // Principals. Obsolete.
                jsapi::OnNewGlobalHookOption::FireOnNewGlobalHook, // Allow debugger to activate immediately.
                &jsapi::CompartmentOptions::default() // Compartment options. TODO: investigate.
            );
            let g_rooted = Rooted::new(g, parent.context());

            assert!(!g.is_null(), "Could not build JS global object");

            Environment {
                parent: parent.clone(),
                global: g_rooted,
            }
        }
    }

    pub fn null_value(&mut self) -> RootedVal {
        Rooted::new(jsval::NullValue(), self.parent.context())
    }

    pub fn parse_json(&mut self, s: &str) -> Option<RootedVal> {
        unsafe {
            let mut r = self.null_value();
            let ctx = self.parent.context();
            let _c = JSAutoCompartment::new(ctx, self.global.inner.ptr);
            // TODO: str len check
            let u16str = Vec::from_iter(s.encode_utf16());
            match jsapi::JS_ParseJSON(
                ctx, u16str.as_ptr(), u16str.len() as u32, r.handle_mut().inner) {
                true => Some(r),
                // TODO: exception handling?
                false => None,
            }
        }
    }

    fn evaluate_script(&mut self, script: &str, scriptname: &str) -> bool {
        let script_utf16: Vec<u16> = script.encode_utf16().collect();
        let scriptname_cstr = CString::new(scriptname.as_bytes()).unwrap();

        let script_ptr;
        let script_len; // Needs to be c_uint although evaluate takes a size_t. I think?
        if script_utf16.len() == 0 {
            script_ptr = (&[]).as_ptr();
            script_len = 0;
        } else {
            script_ptr = script_utf16.as_ptr();
            script_len = script_utf16.len() as c_uint;
        }

        let mut r = self.null_value();
        let ctx = self.parent.context();
        let _c = JSAutoCompartment::new(ctx, self.global.inner.ptr);
        let options = rust::CompileOptionsWrapper::new(ctx, scriptname_cstr.as_ptr(), 0);

        unsafe {
            if !jsapi::Evaluate2(ctx, options.ptr, script_ptr as *const u16,
                script_len as size_t, r.handle_mut().inner) {
                // maybe_resume_unwind(); // TODO: ???
                false
            } else {
                // TODO: what is the script result?
                true
            }
        }
    }
}
//
// struct ScriptLookup<S> {
//     source: S,
//     name: String,
// }
//
// struct Processor {
//     inner: Environment,
// }
//
// // TODO: don't copy the string
// // TODO: error type?
// pub fn lookup_script(s: ScriptLookup) -> Result<String, io::Error> {
//     match s.source {
//         System => (),
//     }
// }
//
// impl Processor {
//     fn init(script: &ScriptLookup) -> Result<Self, io::Error> {
//
//     }
// }

#[cfg(test)]
mod test {
    use super::RuntimeHandle;

    #[test]
    fn context_smoke_test() {
        RuntimeHandle::new();
    }

    #[test]
    fn json_smoke_test() {
        let mut r = RuntimeHandle::new();
        let mut env = r.new_environment();
        env.parse_json("{}");

        env.parse_json(r#"{"rpc": "2.0", "fn": "add", "callback": true, "params": [42, 23], "id": 1,
        "callback_to": {"site": "www.foo.bar", "port": 8888}}"#);
        // TODO test result
    }
}
