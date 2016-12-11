use std::cell::{RefCell, RefMut};
use std::io::stderr;
use std::ptr::Unique;
use std::rc::Rc;

use js::jsapi::{self, JSContext, JSRuntime};

use thunderhead_store::TdError;

use engine::{LoggingErrorReporter, spidermonkey, traits};

use super::context;

pub struct EngineInner {
    inner: Unique<JSContext>,
    inner_runtime: Unique<JSRuntime>,
}

impl Drop for EngineInner {
    fn drop(&mut self) {
        unsafe {
            jsapi::JS_EndRequest(self.inner.get_mut());
            jsapi::JS_DestroyRuntime(self.inner_runtime.get_mut());
        }
    }
}

pub struct Engine {
    // We basically use this RefCell as a checked UnsafeCell.
    inner: Rc<RefCell<EngineInner>>,
}

pub fn new_engine(js_runtime: &mut JSRuntime, js_context: &mut JSContext) -> Engine {
    unsafe {
        let inner = EngineInner {
            inner_runtime: Unique::new(js_runtime),
            inner: Unique::new(js_context),
        };

        Engine {
            inner: Rc::new(RefCell::new(inner)),
        }
    }
}

// so that clone isn't public
pub fn clone_engine(e: &Engine) -> Engine {
    Engine {
        inner: e.inner.clone(),
    }
}

pub fn js_context(e: &mut Engine) -> &mut JSContext {
    unsafe {
        let mut b = e.inner.borrow_mut();
        let p: *mut JSContext = b.inner.get_mut();
        &mut *p
    }
}

impl traits::Engine for Engine {
    type ActiveContext = super::active_context::ActiveContext;
    type Context = spidermonkey::context::Context;
    type Factory = spidermonkey::factory::Factory;
    type FactoryHandle = spidermonkey::factory::FactoryHandle;
    type Value = spidermonkey::value::RootedVal;

    fn new_context(&mut self) -> Result<Self::Context, TdError> {
        Ok(context::new_context(self))
    }
}
