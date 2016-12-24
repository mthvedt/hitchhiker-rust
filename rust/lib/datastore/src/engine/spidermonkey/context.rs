use std::{mem, ptr};

use js::{jsapi, rust};

// TODO: need a thunderhead_lib crate
use thunderhead_store::TdError;

use engine::traits;

use super::{engine, value};
use super::active_context::{self, compile_file, eval_file};
use super::spec::Spec;

pub struct Context {
    /// Our copy of an Engine.
    /// It turns out Engines are just shared references to EngineInners.
    parent: engine::Engine,
    global: value::RootedObj,
    /// The script that creates the Td object.
    td_script: value::RootedScript,
}

fn new_active_context(e: &mut engine::Engine, g: &mut value::RootedObj) -> active_context::ActiveContext {
    unsafe {
        let cpt = jsapi::JSAutoCompartment::new(engine::js_context(e), value::rooted_inner(g).ptr);

        active_context::new_active_context(e, cpt)
    }
}

pub fn engine(cx: &mut Context) -> &mut engine::Engine {
    &mut cx.parent
}

pub fn new_context(parent: &mut engine::Engine, base: &[u8]) -> Result<Context, TdError> {
    unsafe {
        let mut engine = engine::clone_engine(parent);
        let mut g_rooted;
        let mut td_script;

        {
            let mut cx = engine::js_context(&mut engine);

            let g = jsapi::JS_NewGlobalObject(cx,
                &rust::SIMPLE_GLOBAL_CLASS, // Default global class. TODO: investigate.
                ptr::null_mut(), // Principals. Obsolete.
                jsapi::OnNewGlobalHookOption::FireOnNewGlobalHook, // Allow debugger to activate immediately.
                &jsapi::CompartmentOptions::default() // Compartment options. TODO: investigate.
            );

            assert!(!g.is_null(), "Could not build JS global object"); // TODO record error instead

            g_rooted = value::new_rooted(g, cx);
        }

        {
            let mut acx = new_active_context(&mut engine, &mut g_rooted);

            td_script = try!(compile_file(&mut acx, "td/Td.js".as_ref()));
        }

        Ok(Context {
            parent: engine,
            global: g_rooted,
            td_script: td_script,
        })
    }
}

impl traits::Context<Spec> for Context {
    fn exec<R, F: FnOnce(&mut active_context::ActiveContext) -> R>(&mut self, f: F) -> R {
        unsafe {
            (f)(&mut new_active_context(&mut self.parent, &mut self.global))
        }
    }
}
