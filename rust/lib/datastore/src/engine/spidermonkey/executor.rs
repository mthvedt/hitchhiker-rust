use js::jsapi;

use thunderhead_store::TdError;

use engine::traits;

use super::{context, engine, spec, value};
use super::jscontext::JsContext;

pub struct Executor {
    inner: JsContext,
}

pub fn new_executor(parent: &mut engine::Engine) -> Executor {
    Executor {
        inner: JsContext::new(parent),
    }
}

impl traits::Executor<spec::Spec> for Executor {
    // We used nested contexts for security (although we are executing arbitrary user code here,
    // so what is security really? Just making sure the user can't mess with our system scripts.)
    //
    // It might be faster/easier to use a single context,
    // and use some mechanism to keep Thunderhead's system scripts private.
    // Must investigate further.

    // TODO: return a Rust value of some kind
    fn exec(&mut self, cx: &mut context::Context) -> Result<value::RootedVal, TdError> {
        return context::inner_context(cx).exec(|outer_cx, outer_obj| {
            self.inner.exec(|inner_cx, _| {
                unsafe {
                    // TODO: how to unwrap?
                    let outer_handle = value::handle_mut_from_rooted(outer_obj);
                    if !jsapi::JS_WrapObject(outer_cx, outer_handle.inner) {
                        return Err(TdError::EvalError);
                    }
                    // todo: pass in outer_obj
                    panic!("TODO");
                }
            })
        })
        // Use the outer context
    }
}
