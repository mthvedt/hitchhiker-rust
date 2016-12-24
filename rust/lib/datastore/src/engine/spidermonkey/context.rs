use std::ptr;

use js::{jsapi, rust};

// TODO: need a thunderhead_lib crate
use thunderhead_store::TdError;

use engine::traits;

use super::{active_context, engine, value};
use super::spec::Spec;

pub struct Context {
    parent: engine::Engine,
    global: value::RootedObj,
}

pub fn engine(cx: &mut Context) -> &mut engine::Engine {
    &mut cx.parent
}

pub fn new_context(parent: &mut engine::Engine, base: &[u8]) -> Result<Context, TdError> {
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

        let mut c = Context {
            parent: engine,
            global: g_rooted,
        };

        c.load(base).map(|_| c)
    }
}

impl Context {
    fn load(&mut self, base: &[u8]) -> Result<(), TdError> {
        use super::active_context::eval_file;
        use engine::traits::Context;

        self.exec(|acx| {
            eval_file(acx, "td/Td.js".as_ref())
            .and_then(|_| eval_file(acx, base))
            .map(|_| ())
        })
    }
}

impl traits::Context<Spec> for Context {
    fn exec<R, F: FnOnce(&mut active_context::ActiveContext) -> R>(&mut self, f: F) -> R {
        let cpt = {
            let jcx = engine::js_context(&mut self.parent);
            jsapi::JSAutoCompartment::new(jcx, value::rooted_inner(&mut self.global).ptr)
        };

        (f)(&mut active_context::new_active_context(self, cpt))
    }
}
