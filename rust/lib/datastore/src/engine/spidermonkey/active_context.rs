use std::{io, ptr};
use std::cell::{RefMut};
use std::ffi::CString;

use js::{jsval, rust};
use js::jsapi::{self, HandleValueArray, JSAutoCompartment, JSContext, ReadOnlyCompileOptions};
use libc::{c_uint, size_t};

use thunderhead_store::TdError;

use engine::traits;
use engine::error::{Exception, LoggingErrorReporter};

use super::context::ContextEnv;
use super::engine::{self, Engine};
use super::factory;
use super::globals::ActiveGlobals;
use super::spec::Spec;
use super::value::{self, HandleVal, RootedObj, RootedScript, RootedVal, inner_ref};

pub struct Script {
    // TODO: can/should we use path in error reporting?
    _path: Vec<u8>,
    jsval: value::RootedScript,
}

/// Executes a function. If it returns false, checks for exceptions and returns an EvalError.
fn call_jsapi<F, R>(f: F, acx: &mut ActiveContextInner, mut r: R) -> Result<R, TdError> where
F: FnOnce(&mut ActiveContextInner, &mut R) -> bool,
{
    if (f)(acx, &mut r) {
        Ok(r)
    } else {
        acx.check_exception();
        Err(TdError::EvalError)
    }
}

/// Helper trait to specialize load_file.
trait LoadMode {
    type Result;

    fn load(path: &[u8],
        cx: &mut ActiveContextInner,
        options: &ReadOnlyCompileOptions,
        chars: &[u16],
        len: usize) -> Result<Self::Result, TdError>;
}

enum CompileOnly {}
enum Eval {}

impl LoadMode for CompileOnly {
    type Result = Script;

    fn load(path: &[u8],
        acx: &mut ActiveContextInner,
        options: &ReadOnlyCompileOptions,
        chars: &[u16],
        len: usize) -> Result<Self::Result, TdError>
    {
        let script = acx.null_script();

        call_jsapi(
            |acx, mut script| unsafe {
                jsapi::Compile2(
                    &mut *acx.js_context(),
                    options,
                    chars.as_ptr(),
                    len,
                    value::handle_mut_from_rooted(&mut script).inner
                )
            }, acx, script
        ).map(|script| {
            Script {
                _path: Vec::from(path),
                jsval: script
            }
        })
    }
}

impl LoadMode for Eval {
    type Result = value::RootedVal;

    // TODO: should we use _path as error reporting?
    fn load(_path: &[u8],
        acx: &mut ActiveContextInner,
        options: &ReadOnlyCompileOptions,
        chars: &[u16],
        len: usize) -> Result<Self::Result, TdError>
    {
        let r = acx.null_value();

        call_jsapi(
            |acx, mut r| unsafe {
                jsapi::Evaluate2(
                    &mut *acx.js_context(),
                    options,
                    chars.as_ptr(),
                    len,
                    value::handle_mut_from_rooted(&mut r).inner
                )
            }, acx, r
        )
    }
}

// TODO: rename to something like JsManager
/// Container/manager for a Javascript execution context.
/// Spidermonkey contexts should only be used via one of these.
pub struct ActiveContextInner {
    /// The parent Javascript engine.
    /// This is really a borrowed &mut Engine, but since this is a member of an associated type,
    /// we can't lifetime it.

    // TODO: due to LLVM's aliasing rules, it's illegal to have two pointers to the same EngineInner
    // or any part of it in the same stack frame. Right now we don't enforce that, but we should.
    parent: *mut Engine,
    /// This is really a borrow also.
    global_obj: *mut value::RootedObj,

    // Used for JS scoping.
    active_globals: ActiveGlobals,
    _cpt: JSAutoCompartment,
}

impl ActiveContextInner {
    pub fn new(parent: &mut Engine, global_obj: &mut value::RootedObj) -> Self {
        let active_globals;
        let cpt;
        {
            let mut jcx = engine::js_context(parent);
            cpt = jsapi::JSAutoCompartment::new(&mut *jcx, *value::inner_ref(global_obj));
            active_globals = ActiveGlobals::set_scoped(&mut *jcx, LoggingErrorReporter);
        }

        ActiveContextInner {
            parent: parent,
            global_obj: global_obj,
            // TODO: custom error reporters
            active_globals: active_globals,
            // global: &mut self.global,
            _cpt: cpt,
        }
    }

    pub fn js_context<'a>(&'a mut self) -> RefMut<'a, JSContext> {
        engine::js_context(self.parent_engine())
    }

    fn parent_engine(&mut self) -> &mut Engine {
        unsafe { &mut *self.parent }
    }

    fn new_object(&mut self) -> RootedObj {
        // second argument is class--null class means vanilla object
        unsafe {
            let obj = jsapi::JS_NewObject(&mut *self.js_context(), ptr::null_mut());
            value::new_rooted(obj, &mut *self.js_context())
        }
    }

    fn null_value(&mut self) -> RootedVal {
        value::new_rooted(jsval::NullValue(), &mut *self.js_context())
    }

    fn null_script(&mut self) -> RootedScript {
        value::new_rooted(ptr::null_mut(), &mut *self.js_context())
    }

    fn check_exception(&mut self) {
        let mut ex = self.null_value();
        unsafe {
            if jsapi::JS_GetPendingException(&mut *self.js_context(), value::handle_mut_from_rooted(&mut ex).inner) {
                jsapi::JS_ClearPendingException(&mut *self.js_context());
                // TODO handle error
                let eobj = Exception { message: value::rooted_val_to_string(&ex, self, true).unwrap() };
                // TODO: don't panic on err, instead report and exit!

                let jcx;
                { jcx = &*self.js_context() as *const _; }
                self.active_globals.report_exception(jcx, eobj);
            }
        }
    }

    /// HandleValueArray doesn't need to be mut for API purposes, but it should be mut to prevent the the
    /// (unlikely) possibility of aliasing unsafety.
    fn call_fval(&mut self, fobj: HandleVal, args: &mut HandleValueArray) -> Result<RootedVal, TdError> {
        // TODO: what is the right function object to pass?
        let thisobj = self.new_object();
        let r = self.null_value();

        call_jsapi(
            |acx, mut r| unsafe {
                jsapi::JS_CallFunctionValue(
                    &mut *acx.js_context(),
                    value::handle_from_rooted(&thisobj).inner, // Function object (aka `this`). TODO: is this correct?
                    fobj.inner, // The function itself.
                    args,
                    value::handle_mut_from_rooted(&mut r).inner,
                )
            }, self, r
        )
    }

    fn exec_script(&mut self, script: &mut Script) -> Result<RootedVal, TdError> {
        let r = self.null_value();

        call_jsapi(
            |acx, mut r| unsafe {
                jsapi::JS_ExecuteScript(
                    &mut *acx.js_context(),
                    value::handle_from_rooted(&script.jsval).inner,
                    value::handle_mut_from_rooted(&mut r).inner,
                )
            }, self, r
        )
    }

    fn load<T: LoadMode>(&mut self, path: &str, source: &[u8]) -> Result<T::Result, TdError> {
        let script_utf16: Vec<u16> = String::from_utf8_lossy(source).encode_utf16().collect();
        let name_cstr = CString::new(path.as_bytes()).unwrap();
        let script_slice: &[u16];
        let script_len; // Needs to be c_uint although evaluate takes a size_t. I think?

        if script_utf16.len() == 0 {
            script_slice = &[];
            script_len = 0;
        } else {
            script_slice = script_utf16.as_slice();
            script_len = script_utf16.len() as c_uint;
        }

        let options = rust::CompileOptionsWrapper::new(&mut *self.js_context(), name_cstr.as_ptr(), 0);

        T::load(path.as_ref(), self, unsafe { &*options.ptr }, script_slice, script_len as size_t)
    }

    fn load_file<T: LoadMode>(&mut self, path: &[u8]) -> Result<T::Result, TdError> {
        engine::exec_for_factory_handle(
            self.parent_engine(),
            |h| factory::inner(h).user_store.load(path)
        )
        .and_then(|opt|
            opt.ok_or(TdError::new_io(io::ErrorKind::NotFound,
                format!("Source file \'{}\' not found", String::from_utf8_lossy(path)))))
        // TODO: real errors
        .and_then(|s| {
            // TODO: error if path is not a valid str? Should we require strs everywhere?
            // TODO: real errors
            (*s).get().ok_or(TdError::EvalError).and_then(
                |s| self.load::<T>(String::from_utf8_lossy(path).as_ref(), s))
        })
    }

    pub fn eval_file(&mut self, name: &[u8]) -> Result<value::RootedVal, TdError> {
        self.load_file::<Eval>(name)
    }

    pub fn compile_file(&mut self, name: &[u8]) -> Result<Script, TdError> {
        self.load_file::<CompileOnly>(name)
    }
}

pub struct ActiveContext {
    inner: ActiveContextInner,

    /// The parent Context's environment.
    /// This is really a borrowed &mut ContextEnv, but since this is a member of an associated type,
    /// we can't lifetime it.
    // TODO: find a safer way to do this.
    parent_env: *mut ContextEnv,
}

pub fn new_active_context(inner: ActiveContextInner, parent_env: &mut ContextEnv) -> ActiveContext {
    ActiveContext {
        inner: inner,
        parent_env: parent_env,
    }
}

// TODO: this is only used by Value. Value's functionality should probably moved into here
pub fn inner(cx: &mut ActiveContext) -> &mut ActiveContextInner {
    &mut cx.inner
}

impl traits::ActiveContext<Spec> for ActiveContext {
    fn get_schema(&mut self) -> Result<value::RootedVal, TdError> {
        // To clarify what's happening here, JSVals are always just pointers or raw values.
        // It's safe to copy the JSVal as long as it's rooted in the same scope.
        // So we copy the pointers and/or raw values into an array, a HandleValueArray, and pass it
        // to SpiderMonkey.
        let globalobj_val = unsafe { jsval::ObjectOrNullValue(inner_ref(&mut *self.inner.global_obj).clone()) };
        let mut td = try!(self.inner.exec_script(unsafe { &mut (*self.parent_env).td_script }));
        let tdval = inner_ref(&mut td).clone();
        let f = try!(self.inner.load::<Eval>(
            "system://get_store.js",
            include_str!("js/system/get_store.js").as_ref()));

        let arr: [jsapi::Value; 2] = [globalobj_val, tdval];
        let mut args = unsafe { jsapi::HandleValueArray::from_rooted_slice(&arr) };

        self.inner.call_fval(value::handle_from_rooted(&f), &mut args)
    }

    //
    // fn eval_fn(&mut self, f: &mut value::RootedVal, value_bytes: &[u8]) -> Result<value::RootedVal, TdError>
    // {
    //     unsafe {
    //         // TODO: use JSString directly instead?
    //         let scow = String::from_utf8_lossy(value_bytes);
    //         let mut arg = self.null_value();
    //         // TODO: str len check
    //         let u16str = Vec::from_iter(scow.encode_utf16());
    //
    //         if jsapi::JS_ParseJSON(
    //             self.js_context, u16str.as_ptr(), u16str.len() as u32, value::handle_mut_from_rooted(&mut arg).inner) {
    //             let args = jsapi::HandleValueArray { length_: 1, elements_: &value::inner_rooted(&arg).ptr, };
    //             self.call_fval(&value::handle_from_rooted(f), &args)
    //         } else {
    //             self.check_exception();
    //             Err(TdError::EvalError)
    //         }
    //     }
    // }
}
