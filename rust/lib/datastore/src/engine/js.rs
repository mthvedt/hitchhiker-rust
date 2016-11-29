use std;
use std::{io, mem, ptr, slice};
use std::cell::{Cell, RefCell};
use std::ffi::{CStr, CString};
use std::io::Write;
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::ptr::Unique;
use std::rc::Rc;

use libc::{c_char, c_uint, size_t};

use thunderhead_store::{StringSource, TdError};
use thunderhead_store::alloc;
use thunderhead_store::tdfuture::{BoxFuture, FutureExt};

use js::jsapi;
use js::jsapi::{
    HandleValueArray,
    JSAutoCompartment, JSContext, JSErrorReport, JSFunction, JSObject, JSRuntime,
    Value
};
use js::jsval;
use js::rust;

// TODO: name. Make this not dependent on SpiderMonkey.
pub struct Rooted<T> {
    inner: jsapi::Rooted<T>,
}

pub type RootedFn = Rooted<*mut JSFunction>;
pub type RootedObj = Rooted<*mut JSObject>;
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

impl RootedVal {
    // TODO: make the context compartment safe
    fn to_string(&self, cx: &mut JSContext) -> Result<String, TdError> {
        unsafe {
            if self.inner.ptr.is_string() {
                let js_str = self.inner.ptr.to_string();
                // TODO: write to buffer instead
                let mut buf = [0 as u8; 65536];
                let v = jsapi::JS_EncodeStringToBuffer(cx, js_str, &mut buf[0] as *mut u8 as *mut i8, 65536);

                if v > 65536 {
                    Err(TdError::RuntimeError(String::from("string too big")))
                } else if v == (0 as size_t).wrapping_sub(1) {
                    Err(TdError::RuntimeError(String::from("could not encode string")))
                } else {
                    Ok(String::from_utf8_lossy(&buf[..v]).into_owned())
                }
            } else {
                Err(TdError::RuntimeError(String::from("failed string cast")))
            }
        }
    }
}

impl<T> Drop for Rooted<T> {
    fn drop(&mut self) {
        // TODO: how does rooting work in rust?
        // unsafe {
        //     self.inner.remove_from_root_stack()
        // }
    }
}

struct Handle<'a, T: 'a> {
    inner: jsapi::Handle<T>,
    _p: PhantomData<&'a T>,
}

type HandleVal<'a> = Handle<'a, Value>;

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

pub enum ErrorType {
    Error = 0x0,
    Warning = 0x1,
    UncaughtException = 0x2,
}

// TODO: should not be js-specific
// TODO: pretty-printing for errors
#[derive(Debug)]
pub struct Error {
    filename: String,
    line: String,
    lineno: c_uint,
    column: c_uint,
    is_muted: bool,
    // TODO: why two messages?
    message: String,
    message2: String,
    js_flags: c_uint,
    js_errno: c_uint,
    js_exntype: i16,
}

impl Error {
    fn from_js(message: *const c_char, report: *const JSErrorReport) -> Error {
        // TODO: JSREPORT_EXCEPTION?
        assert!(report != ptr::null());
        let report = unsafe { &*report };
        assert!(!report.isMuted); // We don't know how to handle this yet

        let message2_len = 65536;

        Error {
            filename: unsafe { CStr::from_ptr(report.filename).to_string_lossy().into_owned() },
            line: String::from_utf16_lossy(unsafe { slice::from_raw_parts( report.linebuf_, report.linebufLength_) }),
            lineno: report.lineno,
            column: report.column,
            is_muted: report.isMuted,
            js_flags: report.flags,
            js_errno: report.errorNumber,
            message: unsafe { CStr::from_ptr(message).to_string_lossy().into_owned() },
            // TODO: We have to use JS to parse the error message. Yuck.
            // TODO: incredibly unsafe! probably remove this!
            message2: String::from_utf16_lossy(unsafe { slice::from_raw_parts( report.ucmessage, message2_len) }),
            js_exntype: report.exnType,
        }
    }
}

trait ErrorReporter {
    fn is_empty(&self) -> bool;
    fn report(&mut self, e: Error);
}

/// An error reporter that logs to stderr. It never takes ownership of errors, so it is always empty.
/// Useful for bootstrapping.
// TODO: should probably log to a master 'TDContext' instead.
struct LoggingErrorReporter;

impl ErrorReporter for LoggingErrorReporter {
    fn is_empty(&self) -> bool {
        true
    }

    fn report(&mut self, e: Error) {
        writeln!(&mut std::io::stderr(), "{:?}", e).ok();
    }
}

// // TODO: eventually, this should be configurable
// const ERROR_QUEUE_MAX: usize = 20;
//
// struct ErrorQueue {
//     warnings: Vec<Error>,
//     extra_warnings: usize,
//     errors: Vec<Error>,
//     extra_errors: usize,
// }
//
// impl ErrorQueue {
//     fn new() -> Self {
//         ErrorQueue {
//             warnings: Vec::with_capacity(ERROR_QUEUE_MAX),
//             extra_warnings: 0,
//             errors: Vec::with_capacity(ERROR_QUEUE_MAX),
//             extra_errors: 0,
//         }
//     }
//
//     fn is_empty(&self) -> bool {
//         self.inner.len() == 0
//     }
//
//     fn drain_errors(&mut self) -> Vec<Error> {
//         let mut r = Vec::with_capacity(ERROR_QUEUE_MAX);
//         mem::swap(&mut r, &mut self.errors);
//         self.saturated = false;
//
//         r
//     }
// }
//
// impl ErrorReporter for ErrorQueue {
//     fn push(&mut self, e: Error) {
//         if self.inner.len() > ERROR_QUEUE_MAX {
//             self.saturated = true;
//         } else if is_error {
//             self.errors.push(e);
//         }
//     }
// }

thread_local! {
    // Safety check. Must be equal to the executing context.
    // TODO: we will probably need this to be a stack.
    static CURRENT_CONTEXT: Cell<*const JSContext> = Cell::new(ptr::null());

    // The error reporter for the currently executing context. Must be set while JS is executing.
    // TODO: we will probably need this to be an 'option stack', where contexts may optionally push/pop.
    static CURRENT_ERROR_REPORTER: RefCell<Option<Box<ErrorReporter>>> = RefCell::new(None);
}

struct ContextGlobals;

impl ContextGlobals {
    fn set_scoped<E: ErrorReporter + 'static>(cx_p: *const JSContext, reporter: E) -> ContextGlobals {
        assert!(CURRENT_CONTEXT.with(|cx_p_cell| cx_p_cell.get() == ptr::null()));
        assert!(CURRENT_ERROR_REPORTER.with(|r| r.borrow().is_none()));

        CURRENT_CONTEXT.with(|cx_p_cell| cx_p_cell.set(cx_p));
        CURRENT_ERROR_REPORTER.with(|r| *r.borrow_mut() = Some(Box::new(reporter)));

        ContextGlobals
    }
}

impl Drop for ContextGlobals {
    fn drop(&mut self) {
        CURRENT_CONTEXT.with(|cx_p_cell| cx_p_cell.set(ptr::null()));
        CURRENT_ERROR_REPORTER.with(|r| {
            match r.try_borrow_mut() {
                Ok(mut rref) => match mem::replace(&mut *rref, None) {
                    Some(r) => if r.is_empty() {
                        // We use ok()--if we can't writeln to stderr, there's no hope for us!
                        writeln!(&mut std::io::stderr(), "WARNING: javascript errors ignored during unwinding").ok();
                        // TODO: print errors to stderr
                    },
                    None => {
                        writeln!(&mut std::io::stderr(),
                            "WARNING: javascript reporter in invalid state during unwinding").ok();
                    }
                },
                Err(_) => {
                    // Panic during drop (probably terminating the program), but explain why
                    writeln!(&mut std::io::stderr(),
                        "FATAL: couldn't access javascript error queue during unwinding").ok();
                    panic!();
                },
            }
        });
    }
}

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
    extern "C" fn report_warning(cx: *mut JSContext, message: *const c_char, report: *mut JSErrorReport) {
        // The current context and error queue *must* be set.
        // TODO: when ptr_eq is stable, use that
        assert!(CURRENT_CONTEXT.with(|cx_p_cell| cx_p_cell.get() == cx));
        let e = Error::from_js(message, report);
        CURRENT_ERROR_REPORTER.with(|eq_c| eq_c.borrow_mut().as_mut().unwrap().report(e));
    }

    fn new() -> Self {
        unsafe {
            let js_runtime = jsapi::JS_NewRuntime(DEFAULT_HEAP_SIZE, DEFAULT_CHUNK_SIZE, master_runtime());
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

            jsapi::SetWarningReporter(js_runtime, Some(Self::report_warning));

            let _g = ContextGlobals::set_scoped(js_context, LoggingErrorReporter);

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
    pub fn new_runtime() -> Self {
        RuntimeHandle {
            inner: Rc::new(RefCell::new(Runtime::new())),
        }
    }

    pub fn new_environment(&mut self) -> Environment {
        Environment::new(self)
    }

    fn context(&mut self) -> &mut JSContext {
        unsafe {
            let mut b = self.inner.borrow_mut();
            let p: *mut JSContext = b.inner.get_mut();
            &mut *p
        }
    }
}

pub struct Environment {
    parent: RuntimeHandle,
    global: RootedObj,
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

    fn context<'a>(&'a mut self) -> ExecContext<'a> {
        let cx = self.parent.context();
        let cxp = cx as *const _;
        let cpt = JSAutoCompartment::new(cx, self.global.inner.ptr);

        ExecContext {
            jscontext: cx,
            // global: &mut self.global,
            _cpt: cpt,
            // TODO: custom error reporters
            _g: ContextGlobals::set_scoped(cxp, LoggingErrorReporter),
        }
    }
}

pub struct ExecContext<'a> {
    jscontext: &'a mut JSContext,
    // global: &'a mut RootedObj,

    // Used for JS scoping.
    _cpt: JSAutoCompartment,
    _g: ContextGlobals,
}

impl<'a> ExecContext<'a> {
    pub fn new_object(&mut self) -> RootedObj {
        // second argument is class--null class means vanilla object
        unsafe {
            Rooted::new(jsapi::JS_NewObject(self.jscontext, ptr::null()), self.jscontext)
        }
    }

    pub fn null_value(&mut self) -> RootedVal {
        Rooted::new(jsval::NullValue(), self.jscontext)
    }

    pub fn parse_json<Bytes: alloc::Scoped<[u8]>>(&mut self, b: Bytes) -> Option<RootedVal> {
        unsafe {
            // TODO: use JSString directly instead?
            let scow = String::from_utf8_lossy(b.get().unwrap());
            let mut r = self.null_value();
            // TODO: str len check
            let u16str = Vec::from_iter(scow.encode_utf16());
            match jsapi::JS_ParseJSON(
                self.jscontext, u16str.as_ptr(), u16str.len() as u32, r.handle_mut().inner) {
                true => Some(r),
                // TODO: exception handling?
                false => None,
            }
        }
    }

    fn evaluate_script(&mut self, script: &str, scriptname: &str) -> Result<RootedVal, TdError> {
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
        let options = rust::CompileOptionsWrapper::new(self.jscontext, scriptname_cstr.as_ptr(), 0);

        unsafe {
            if jsapi::Evaluate2(self.jscontext, options.ptr, script_ptr as *const u16,
                script_len as size_t, r.handle_mut().inner) {
                // maybe_resume_unwind(); // TODO: ???
                Ok(r)
            } else {
                // TODO: what is the script result?
                Err(TdError::EvalError)
            }
        }
    }

    fn call_fval_one(&mut self, fobj: &HandleVal, arg: &RootedVal) -> Result<RootedVal, TdError> {
        let args = jsapi::HandleValueArray { length_: 1, elements_: &arg.inner.ptr, };
        self.call_fval(fobj, &args)
    }

    fn call_fval(&mut self, fobj: &HandleVal, args: &HandleValueArray) -> Result<RootedVal, TdError> {
        // TODO: what is the right function object to pass?
        let thisobj = self.new_object();
        let mut r = self.null_value();

        unsafe {
            jsapi::JS_CallFunctionValue(
                self.jscontext,
                thisobj.handle().inner, // Function object (aka `this`). TODO: is this correct?
                fobj.inner, // The function itself.
                args,
                r.handle_mut().inner,
            )
        };

        // TODO: errors
        Ok(r)
    }
}

pub struct Processor {
    env: Environment,
    /// Actually a function mapping (what to what)?
    f: RootedVal,
}

impl Processor {
    fn new(env: Environment, f: RootedVal) -> Result<Self, TdError> {
        // TODO: verify f. Separate verifier environment?
        // TODO: also verify (debug assert?) f belongs to environment.

        Ok(Processor {
            env: env,
            f: f,
        })
    }

    pub fn from_source<Source, Str>(mut env: Environment, name: Str, mut source: Source) ->
    BoxFuture<Self, TdError> where
    Source: StringSource,
    Str: alloc::Scoped<str> + 'static
    {
        source.get(name.get().unwrap().as_ref()).lift(move |scriptopt| {
            use thunderhead_store::alloc::Scoped; // This breaks get() type inference for some reason.

            let script_result = scriptopt.ok_or(TdError::from(
                io::Error::new(io::ErrorKind::NotFound, "Script not found")));
            // TODO: better names
            script_result.and_then(|script| {
                let fresult = {
                    let mut cx = env.context();
                    // TODO: an 'unscope' macro
                    cx.evaluate_script(script.get().unwrap(), name.get().unwrap())
                };
                fresult.and_then(|f| Self::new(env, f))
            })
        }).td_boxed()
    }

    pub fn apply<Bytes: alloc::Scoped<[u8]>>(&mut self, value_bytes: Bytes) -> Result<RootedVal, TdError> {
        let mut cx = self.env.context();
        let fmut = self.f.handle();

        let call_value = match cx.parse_json(value_bytes.get().unwrap()) {
            Some(v) => v,
            // TODO better error handling
            None => return Err(TdError::EvalError),
        };

        cx.call_fval_one(&fmut, &call_value)
    }

    pub fn to_string(&mut self, str: RootedVal) -> Result<String, TdError> {
        let mut cx = self.env.context();
        str.to_string(cx.jscontext)
    }
}

#[cfg(test)]
mod test {
    use super::RuntimeHandle;

    #[test]
    fn runtime_smoke_test() {
        RuntimeHandle::new_runtime().new_environment().context();
    }

    #[test]
    fn json_smoke_test() {
        let mut r = RuntimeHandle::new_runtime();
        let mut env = r.new_environment();
        let mut cx = env.context();
        cx.parse_json("{}".as_ref());

        cx.parse_json(r#"{"rpc": "2.0", "fn": "add", "callback": true, "params": [42, 23], "id": 1,
        "callback_to": {"site": "www.foo.bar", "port": 8888}}"#.as_ref());
        // TODO test result
    }
}
