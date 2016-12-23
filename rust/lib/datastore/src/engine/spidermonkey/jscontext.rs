use std::ptr;

use js::{jsapi, rust};

use thunderhead_store::TdError;

use engine::error::LoggingErrorReporter;
use engine::traits;

use super::{active_context, engine, globals, value};
use super::spec::Spec;

/// JsContext: a context used to handle Contexts and Executors.
pub struct JsContext {
    parent: engine::Engine,
    global: value::RootedObj,
}

// Safe because we never export JsContext.
impl JsContext {
    pub fn engine(&mut self) -> &mut engine::Engine {
        &mut self.parent
    }

    pub fn new(parent: &mut engine::Engine) -> JsContext {
        unsafe {
            let mut engine = engine::clone_engine(parent);
            let g_rooted;

            {
                let cx = engine::js_context(&mut engine);

                let g = jsapi::JS_NewGlobalObject(cx,
                    &rust::SIMPLE_GLOBAL_CLASS, // Default global class. TODO: investigate.
                    ptr::null_mut(), // Principals. Obsolete.
                    jsapi::OnNewGlobalHookOption::FireOnNewGlobalHook, // Allow debugger to activate immediately.
                    &jsapi::CompartmentOptions::default() // Compartment options. TODO: investigate.
                );

                assert!(!g.is_null(), "Could not build JS global object"); // TODO record error instead

                g_rooted = value::new_rooted(g, cx);
            }

            JsContext {
                parent: engine,
                global: g_rooted,
            }
        }
    }

    pub fn exec<R, F: FnOnce(&mut jsapi::JSContext, &mut value::RootedObj) -> R>(&mut self, f: F) -> R {
        // TODO: custom error reporters
        let _g = globals::ActiveGlobals::set_scoped(engine::js_context(&mut self.parent), LoggingErrorReporter);
        let _cpt = {
            let jcx = engine::js_context(&mut self.parent);
            jsapi::JSAutoCompartment::new(jcx, value::rooted_inner(&mut self.global).ptr)
        };

        (f)(engine::js_context(&mut self.parent), &mut self.global)
    }
}
