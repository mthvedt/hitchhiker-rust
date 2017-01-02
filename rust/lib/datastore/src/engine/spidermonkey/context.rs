use std::ptr;

use js::{jsapi, rust};

// TODO: need a thunderhead_lib crate
use thunderhead_store::TdError;

use engine::traits;

use super::engine;
use super::active_context::{self, ActiveContextInner, Script};
use super::spec::Spec;
use super::value::{self, Rooted};

pub struct ContextEnv {
    /// The script that creates the Td object.
    pub td_script: Script,
}

pub struct Context {
    /// Our copy of an Engine.
    /// It turns out Engines are just shared references to EngineInners.
    /// TODO: something safer. (EngineRef?)
    parent: engine::Engine,
    global: value::RootedObj,
    env: ContextEnv,
}

pub fn new_context(parent: &mut engine::Engine, base: &[u8]) -> Result<Context, TdError> {
    unsafe {
        let mut engine = engine::clone_engine(parent);
        let mut g_rooted;
        let td_script;

        {
            let mut cx = engine::js_context(&mut engine);

            let g = jsapi::JS_NewGlobalObject(&mut *cx,
                &rust::SIMPLE_GLOBAL_CLASS, // Default global class. TODO: investigate.
                ptr::null_mut(), // Principals. Obsolete.
                jsapi::OnNewGlobalHookOption::FireOnNewGlobalHook, // Allow debugger to activate immediately.
                &jsapi::CompartmentOptions::default() // Compartment options. TODO: investigate.
            );

            assert!(!g.is_null(), "Could not build JS global object"); // TODO record error instead

            g_rooted = Rooted::new(g, &mut *cx);
        }

        {
            let mut acx = ActiveContextInner::new(&mut engine, &mut g_rooted);

            td_script = try!(acx.compile_file("td/Td.js".as_ref()));

            try!(acx.eval_file(base));
        }

        Ok(Context {
            parent: engine,
            global: g_rooted,
            env: ContextEnv {
                td_script: td_script,
            },
        })
    }
}

impl traits::Context<Spec> for Context {
    fn exec<R, F: FnOnce(&mut active_context::ActiveContext) -> R>(&mut self, f: F) -> R {
        let acx_inner = ActiveContextInner::new(&mut self.parent, &mut self.global);
        (f)(&mut active_context::new_active_context(acx_inner, &mut self.env))
    }
}
