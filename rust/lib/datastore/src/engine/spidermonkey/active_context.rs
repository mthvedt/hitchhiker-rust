use std::ffi::CString;
use std::iter::FromIterator;
use std::ptr;

use js::jsapi::{self, HandleValueArray, JSAutoCompartment, JSContext};
use js::{jsval, rust};
use libc::{c_uint, size_t};

use thunderhead_store::{TdError, alloc};

use engine::error::{Exception, LoggingErrorReporter};
use engine::traits;

use super::globals::ActiveGlobals;
use super::spec::Spec;
use super::value::{self, HandleVal, RootedObj, RootedVal};

pub struct ActiveContext {
    // TODO: we really want this to be an &'a mut. See comments about HKTs in traits.rs
    js_context: *mut JSContext,

    // Used for JS scoping.
    g: ActiveGlobals,
    _cpt: JSAutoCompartment,
}

pub fn new_active_context(jcx: &mut JSContext, cpt: JSAutoCompartment) -> ActiveContext {
    ActiveContext {
        js_context: jcx as *mut _,
        // TODO: custom error reporters
        g: ActiveGlobals::set_scoped(jcx, LoggingErrorReporter),
        // global: &mut self.global,
        _cpt: cpt,
    }
}

impl ActiveContext {
    fn js_context(&mut self) -> &mut JSContext {
        unsafe { &mut *self.js_context }
    }

    fn new_object(&mut self) -> RootedObj {
        // second argument is class--null class means vanilla object
        unsafe {
            value::new_rooted(jsapi::JS_NewObject(self.js_context(), ptr::null()), self.js_context())
        }
    }

    fn null_value(&mut self) -> RootedVal {
        value::new_rooted(jsval::NullValue(), self.js_context())
    }

    fn check_exception(&mut self) {
        let mut ex = self.null_value();
        unsafe {
            if jsapi::JS_GetPendingException(self.js_context(), value::handle_mut_from_rooted(&mut ex).inner) {
                jsapi::JS_ClearPendingException(self.js_context());
                // TODO handle error
                let eobj = Exception { message: value::rooted_val_to_string(&ex, self, true).unwrap() };
                // TODO: don't panic on err, instead report and exit!
                self.g.report_exception(self.js_context, eobj);
            }
        }
    }

    pub fn parse_json<Bytes: alloc::Scoped<[u8]>>(&mut self, b: Bytes) -> Result<RootedVal, TdError> {
        unsafe {
            // TODO: use JSString directly instead?
            let scow = String::from_utf8_lossy(b.get().unwrap());
            let mut r = self.null_value();
            // TODO: str len check
            let u16str = Vec::from_iter(scow.encode_utf16());

            if jsapi::JS_ParseJSON(
                self.js_context(), u16str.as_ptr(), u16str.len() as u32, value::handle_mut_from_rooted(&mut r).inner) {
                Ok(r)
            } else {
                self.check_exception();
                Err(TdError::EvalError)
            }
        }
    }

    fn evaluate_script(&mut self, scriptname: &str, source: &[u8]) -> Result<RootedVal, TdError> {
        let script_utf16: Vec<u16> = String::from_utf8_lossy(source).encode_utf16().collect();
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
        let options = rust::CompileOptionsWrapper::new(self.js_context(), scriptname_cstr.as_ptr(), 0);

        unsafe {
            if jsapi::Evaluate2(self.js_context(), options.ptr, script_ptr as *const u16,
                script_len as size_t, value::handle_mut_from_rooted(&mut r).inner) {
                // maybe_resume_unwind(); // TODO: ???
                Ok(r)
            } else {
                self.check_exception();
                Err(TdError::EvalError)
            }
        }
    }

    fn call_fval_one(&mut self, fobj: &HandleVal, arg: &RootedVal) -> Result<RootedVal, TdError> {
        let args = jsapi::HandleValueArray { length_: 1, elements_: &value::inner_rooted(&arg).ptr, };
        self.call_fval(fobj, &args)
    }

    fn call_fval(&mut self, fobj: &HandleVal, args: &HandleValueArray) -> Result<RootedVal, TdError> {
        // TODO: what is the right function object to pass?
        let thisobj = self.new_object();
        let mut r = self.null_value();

        let success = unsafe {
            jsapi::JS_CallFunctionValue(
                self.js_context(),
                value::handle_from_rooted(&thisobj).inner, // Function object (aka `this`). TODO: is this correct?
                fobj.inner, // The function itself.
                args,
                value::handle_mut_from_rooted(&mut r).inner,
            )
        };

        if success {
            Ok(r)
        } else {
            self.check_exception();
            Err(TdError::EvalError)
        }
    }
}

pub fn js_context(ac: &mut ActiveContext) -> &mut JSContext {
    ac.js_context()
}

impl traits::ActiveContext<Spec> for ActiveContext {
    fn eval_script(&mut self, name: &str, source: &[u8]) ->
    Result<value::RootedVal, TdError>
    {
        self.evaluate_script(name, source)
    }

    fn eval_fn(&mut self, f: &mut value::RootedVal, value_bytes: &[u8]) ->
    Result<value::RootedVal, TdError> {
        self.parse_json(value_bytes).and_then(|v| self.call_fval_one(&value::handle_from_rooted(f), &v))
    }
}
