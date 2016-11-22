use std;
use std::ptr;
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr::Unique;

use js::jsapi;
use js::jsapi::{JSAutoCompartment, JSClass, JSContext, JSPrincipals, JSRuntime, JSObject, Value};
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

fn master_runtime() -> *mut JSRuntime {
    struct MasterRuntime(*mut JSRuntime);
    unsafe impl Sync for MasterRuntime {}

    lazy_static! {
        static ref INNER: MasterRuntime = {
            unsafe {
                assert!(jsapi::JS_Init(), "FATAL: Could not init JS");
                let runtime = jsapi::JS_NewRuntime(DEFAULT_HEAP_SIZE, DEFAULT_CHUNK_SIZE, ptr::null_mut());
                assert!(!runtime.is_null(), "FATAL: Could not allocate master JS runtime");

                let context = jsapi::JS_GetContext(runtime);
                assert!(!context.is_null(), "FATAL: Could not get master JS context");
                jsapi::InitSelfHostedCode(context);

                MasterRuntime(runtime)
            }
        };
    }

    INNER.0
}

/// A fully-initialized javascript execution context
pub struct Context {
    inner: Unique<JSContext>,
    inner_runtime: Unique<JSRuntime>,
    global: RootedObject,
}

impl Context {
    /// Creates a new `JSRuntime` and `JSContext`.
    pub fn new() -> Context {
        unsafe {
            let js_runtime = jsapi::JS_NewRuntime(DEFAULT_HEAP_SIZE, DEFAULT_CHUNK_SIZE, master_runtime());
            assert!(!js_runtime.is_null(), "Out of memory allocating JS runtime");

            // Unconstrain the runtime's threshold on nominal heap size, to avoid
            // triggering GC too often if operating continuously near an arbitrary
            // finite threshold. This leaves the maximum-JS_malloc-bytes threshold
            // still in effect to cause periodical, and we hope hygienic,
            // last-ditch GCs from within the GC's allocator.
            jsapi::JS_SetGCParameter(
                js_runtime, jsapi::JSGCParamKey::JSGC_MAX_BYTES, std::u32::MAX);

            let js_context = jsapi::JS_GetContext(js_runtime);
            assert!(!js_context.is_null(), "Could not get JS context");

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

            let g = jsapi::JS_NewGlobalObject(js_context,
                &rust::SIMPLE_GLOBAL_CLASS, // Default global class. TODO: investigate.
                ptr::null_mut(), // Principals. Obsolete.
                jsapi::OnNewGlobalHookOption::FireOnNewGlobalHook, // Allow debugger to activate immediately.
                &jsapi::CompartmentOptions::default() // Compartment options. TODO: investigate.
            );
            let g_rooted = Rooted::new(g, &mut *js_context);

            assert!(!g.is_null(), "Could not build JS global object");

            Context {
                inner_runtime: Unique::new(js_runtime),
                inner: Unique::new(js_context),
                global: g_rooted,
            }
        }
    }

    /* End code taken from rust-mozjs */

    // pub fn evaluate_script(&self, glob: HandleObject, script: &str, filename: &str,
    //                        line_num: u32, rval: MutableHandleValue)
    //                 -> Result<(),()> {
    //     let script_utf16: Vec<u16> = script.encode_utf16().collect();
    //     let filename_cstr = ffi::CString::new(filename.as_bytes()).unwrap();
    //     debug!("Evaluating script from {} with content {}", filename, script);
    //     // SpiderMonkey does not approve of null pointers.
    //     let (ptr, len) = if script_utf16.len() == 0 {
    //         static empty: &'static [u16] = &[];
    //         (empty.as_ptr(), 0)
    //     } else {
    //         (script_utf16.as_ptr(), script_utf16.len() as c_uint)
    //     };
    //     assert!(!ptr.is_null());
    //     let _ac = JSAutoCompartment::new(self.cx(), glob.get());
    //     let options = CompileOptionsWrapper::new(self.cx(), filename_cstr.as_ptr(), line_num);

    //     unsafe {
    //         if !Evaluate2(self.cx(), options.ptr, ptr as *const u16, len as size_t, rval) {
    //             debug!("...err!");
    //             maybe_resume_unwind();
    //             Err(())
    //         } else {
    //             // we could return the script result but then we'd have
    //             // to root it and so forth and, really, who cares?
    //             debug!("...ok!");
    //             Ok(())
    //         }
    //     }
    // }

    pub fn null_value(&mut self) -> RootedVal {
        unsafe {
            Rooted::new(jsval::NullValue(), self.inner.get_mut())
        }
    }

    pub fn parse_json(&mut self, s: &str) -> Option<RootedVal> {
        unsafe {
            let _a = JSAutoCompartment::new(self.inner.get_mut(), self.global.inner.ptr);
            // TODO: str len check
            let u16str = Vec::from_iter(s.encode_utf16());
            let mut r = self.null_value();
            match jsapi::JS_ParseJSON(
                self.inner.get_mut(), u16str.as_ptr(), u16str.len() as u32, r.handle_mut().inner) {
                true => Some(r),
                // TODO: exception handling?
                false => None,
            }
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            jsapi::JS_EndRequest(self.inner.get_mut());
            jsapi::JS_DestroyRuntime(self.inner_runtime.get_mut());
        }
    }
}

#[cfg(test)]
mod test {
    use super::Context;

    #[test]
    fn context_smoke_test() {
        Context::new();
    }

    #[test]
    fn json_smoke_test() {
        let mut c = Context::new();
        c.parse_json("{}");
        c.parse_json(r#"{"rpc": "2.0", "fn": "add", "callback": true, "params": [42, 23], "id": 1,
        "callback_to": {"site": "www.foo.bar", "port": 8888}}"#);
    }
}
