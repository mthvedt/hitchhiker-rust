use std::cell::RefMut;
use std::ffi::CString;
use std::io;
use std::ptr;

use futures::Future;
use js::jsapi::{self, HandleValueArray, JSAutoCompartment, JSContext};
use js::{jsval, rust};
use libc::{c_uint, size_t};

use thunderhead_store::{StringSource, TdError, alloc};
use thunderhead_store::tdfuture::BoxFuture;

use engine::error::{Exception, LoggingErrorReporter};
use engine::traits::{self, Engine};

use super::active_context;
use super::engine::{self, EngineInner};
use super::globals::ActiveGlobals;
use super::value::{self, HandleVal, Rooted, RootedObj, RootedVal};

pub struct Context {
    parent: engine::Engine,
    global: value::RootedObj,
}

pub fn new_context(parent: &mut engine::Engine) -> Context {
    unsafe {
        let engine = engine::clone_engine(parent);
        let cx = engine::js_context(engine);
        
        let g = jsapi::JS_NewGlobalObject(cx,
            &rust::SIMPLE_GLOBAL_CLASS, // Default global class. TODO: investigate.
            ptr::null_mut(), // Principals. Obsolete.
            jsapi::OnNewGlobalHookOption::FireOnNewGlobalHook, // Allow debugger to activate immediately.
            &jsapi::CompartmentOptions::default() // Compartment options. TODO: investigate.
        );

        assert!(!g.is_null(), "Could not build JS global object"); // TODO record error instead

        let g_rooted = value::new_rooted(g, cx);

        Context {
            parent: engine,
            global: g_rooted,
        }
    }
}

impl traits::Context for Context {
    type Engine = engine::Engine;

    fn exec<R, F: FnOnce(&mut <Self::Engine as Engine>::ActiveContext) -> R>(&mut self, f: F) -> R {
        let jcx = engine::js_context(&mut self.parent);
        let cpt = jsapi::JSAutoCompartment::new(jcx, value::rooted_inner(&mut self.global).ptr);

        (f)(&mut active_context::new_active_context(jcx, cpt))
    }
}
