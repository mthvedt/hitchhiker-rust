use std;
use std::{io, mem, ptr, slice};
use std::cell::{Cell, RefCell};
use std::ffi::{CStr, CString};
use std::io::Write;
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::ptr::Unique;
use std::rc::Rc;

use futures::{Future, IntoFuture};
use futures::future;
use js::jsapi;
use js::jsapi::{
    HandleValueArray,
    JSAutoCompartment, JSContext, JSErrorReport, JSFunction, JSObject, JSRuntime,
};
use js::jsval;
use js::rust;
use libc::{c_char, c_uint, size_t};

use thunderhead_store::{KvSource, TdError};
use thunderhead_store::alloc;
use thunderhead_store::tdfuture::{BoxFuture, FutureExt};

use super::traits::{ActiveContext, Context, Engine, EngineSpec, Value};

fn eval_script_from_source<E, Source, Str>(mut cx: E::Context, mut name: Str, mut source: Source) ->
impl Future<Item = (E::Context, E::Value), Error = TdError> where
E: EngineSpec,
Source: KvSource,
Str: alloc::Scoped<str> + 'static
{
    source.get(name.get().unwrap().as_ref()).lift(move |scriptopt| {
        use thunderhead_store::alloc::Scoped; // This breaks get() type inference for some reason.

        let script_result = scriptopt.ok_or(TdError::from(
            io::Error::new(io::ErrorKind::NotFound, "Script not found")));
        // TODO: better names
        script_result.and_then(|script| {
            // TODO: an 'unscope' macro for scoped
            let rres = cx.exec(|ac| ac.eval_script(name.get().unwrap(), script.get().unwrap()));
            rres.map(|r| (cx, r))
        })
    })
}

struct ProcessorInner<E: EngineSpec + 'static> {
    cx: E::Context,
    /// Actually a function mapping (what to what)?
    f: E::Value,
}

pub struct ProcessorHandle<E: EngineSpec + 'static> {
    inner: Rc<RefCell<ProcessorInner<E>>>,
}

impl<E: EngineSpec + 'static> ProcessorHandle<E> {
    fn new_processor(mut cx: E::Context, mut f: E::Value) -> Result<Self, TdError> {
        if !f.is_function() {
            // TODO: use s
            let _s = cx.exec(|mut acx| match f.debug_string(acx) {
                Ok(s) => format!("ERROR: {} is not a function", s),
                // TODO propogate interior error
                Err(s) => "ERROR: given value is not a function. Additionally, could not generate debug string for given value".into(),
            });

            return Err(TdError::EvalError);
        }
        // TODO: verify f. Separate verifier environment?
        // TODO: also verify (debug assert?) f belongs to environment.

        let p = ProcessorInner {
            cx: cx,
            f: f,
        };

        let r = ProcessorHandle {
            inner: Rc::new(RefCell::new(p)),
        };

        Ok(r)
    }

    pub fn processor_from_source<Source, Str>(cx: E::Context, name: Str, source: Source) ->
    impl Future<Item = Self, Error = TdError> where
    Source: KvSource + 'static,
    Str: alloc::Scoped<str> + 'static
    {
        // TODO verify type of f
        eval_script_from_source::<E, Source, Str>(cx, name, source).lift(|(cx, f)| Self::new_processor(cx, f))
    }

    // Right now, these return an immediate Done (with an unfortunate alloc).
    // In the future (heh) they will return a Future.
    pub fn apply<B: alloc::Scoped<[u8]>>(&self, bytes: B) -> BoxFuture<E::Value, TdError> {
        // Fun with the borrow checker...
        let p = &mut *self.inner.borrow_mut();
        let f = &mut p.f;
        let cx = &mut p.cx;

        cx.exec(|mut ac| ac.eval_fn(f, bytes.get().unwrap())).into_future().td_boxed()
    }

    pub fn apply_and_write<B: alloc::Scoped<[u8]>>(&self, bytes: B) -> BoxFuture<(E::Value, String), TdError> {
        // TODO: try to avoid reborrowing. Calls to eval_fn are expensive

        // Fun with the borrow checker...
        let p = &mut *self.inner.borrow_mut();
        let f = &mut p.f;
        let cx = &mut p.cx;

        cx.exec(|mut ac| {
            ac.eval_fn(f, bytes.get().unwrap()).and_then(|mut val| {
                val.serialize(ac).map(|val_str| (val, val_str))
            })
        }).into_future().td_boxed()
    }
}

impl<E: EngineSpec + 'static> Clone for ProcessorHandle<E> {
    fn clone(&self) -> Self {
        ProcessorHandle {
            inner: self.inner.clone(),
        }
    }
}
