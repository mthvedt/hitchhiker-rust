use js::jsapi;

use engine::traits;

use super::{active_context, engine, spec, value};
use super::jscontext::JsContext;

pub struct Context {
    inner: JsContext,
}

pub fn new_context(parent: &mut engine::Engine) -> Context {
    Context {
        inner: JsContext::new(parent),
    }
}

pub fn inner_context(cx: &mut Context) -> &mut JsContext {
    &mut cx.inner
}

impl traits::Context<spec::Spec> for Context {
    fn exec<R, F: FnOnce(&mut active_context::ActiveContext) -> R>(&mut self, f: F) -> R {
        panic!("deleteme");

        // let cpt = {
        //     let jcx = engine::js_context(&mut self.parent);
        //     jsapi::JSAutoCompartment::new(jcx, value::rooted_inner(&mut self.global).ptr)
        // };
        //
        // (f)(&mut active_context::new_active_context(self, cpt))
    }
}
