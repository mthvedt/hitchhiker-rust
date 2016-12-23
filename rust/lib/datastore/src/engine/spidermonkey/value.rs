use std::marker::PhantomData;

use js::jsapi::{self, JSContext, JSObject, Value};
use js::rust;
use libc::size_t;

use thunderhead_store::TdError;

use engine::traits;
use engine::value::NativeValue;

use super::active_context::{self, ActiveContext};
use super::spec::Spec;

// TODO: name. Make this not dependent on SpiderMonkey.
pub struct Rooted<T> {
    inner: jsapi::Rooted<T>,
}

pub type RootedObj = Rooted<*mut JSObject>;
pub type RootedVal = Rooted<Value>;

// Because Rust doesn't allow unexported self impls...
pub fn new_rooted<T>(t: T, cx: &mut JSContext) -> Rooted<T> where T: rust::RootKind {
    let mut inner = jsapi::Rooted::new_unrooted(t);
    unsafe {
        // TODO: how does rooting work in rust?
        inner.add_to_root_stack(cx);
    }

    Rooted {
        inner: inner,
    }
}

pub fn inner_rooted<'a, T>(r: &'a Rooted<T>) -> &'a jsapi::Rooted<T> {
    &r.inner
}

pub fn handle_from_rooted<'a, T>(r: &'a Rooted<T>) -> Handle<'a, T> {
    Handle {
        inner: unsafe { jsapi::Handle::from_marked_location(&r.inner.ptr) },
        _p: PhantomData,
    }
}

pub fn handle_mut_from_rooted<'a, T>(r: &'a mut Rooted<T>) -> HandleMut<'a, T> {
    HandleMut {
        inner: unsafe { jsapi::MutableHandle::from_marked_location(&mut r.inner.ptr) },
        _p: PhantomData,
    }
}

pub fn rooted_inner<'a, T>(r: &'a mut Rooted<T>) -> &'a mut jsapi::Rooted<T> {
    &mut r.inner
}

pub fn rooted_val_to_string(val: &RootedVal, cx: &mut ActiveContext, force: bool) -> Result<String, TdError> {
    unsafe {
        let jcx = active_context::js_context(cx);

        if val.inner.ptr.is_string() || force {
            let js_str = if force {
                rust::ToString(jcx, handle_from_rooted(val).inner)
            } else {
                val.inner.ptr.to_string()
            };

            // TODO: write to buffer instead. this is dumb.
            let mut buf = [0 as u8; 65536];
            let v = jsapi::JS_EncodeStringToBuffer(jcx, js_str, &mut buf[0] as *mut u8 as *mut i8, 65536);

            if v > 65536 {
                // Err("string too big".into())
                Err(TdError::EvalError)
            } else if v == (0 as size_t).wrapping_sub(1) {
                // Err("could not encode string".into())
                Err(TdError::EvalError)
            } else {
                Ok(String::from_utf8_lossy(&buf[..v]).into_owned())
            }
        } else {
            // Err("failed string cast".into())
            Err(TdError::EvalError)
        }
    }
}

impl traits::Value<Spec> for RootedVal {
    fn is_function(&self, acx: &mut ActiveContext) -> bool {
        if self.inner.ptr.is_object() {
            unsafe {
                jsapi::JS_ObjectIsFunction(active_context::js_context(acx), self.inner.ptr.to_object())
            }
        } else {
            false
        }
    }

    fn to_native_value(&mut self, acx: &mut ActiveContext) -> Result<NativeValue, TdError> {
        let v = self.inner.ptr;

        let r = if v.is_null() {
            NativeValue::Null
        } else if v.is_boolean() {
            NativeValue::Bool(v.to_boolean())
        } else if v.is_double() {
            NativeValue::Double(v.to_double())
        } else if v.is_int32() {
            NativeValue::Int(v.to_int32())
        } else if v.is_string() {
            NativeValue::String(rooted_val_to_string(self, acx, false).unwrap())
        } else if v.is_object() {
            panic!()
        } else {
            return Err(TdError::EvalError)
            // return Err("inconvertible value".into());
        };

        Ok(r)
    }

    fn debug_string(&mut self, acx: &mut ActiveContext) -> Result<String, TdError> {
        // TODO: shorter strings
        rooted_val_to_string(self, acx, true)
    }

    // TODO: write to bytes
    fn serialize(&mut self, acx: &mut ActiveContext) -> Result<String, TdError> {
        // TODO: shorter strings
        rooted_val_to_string(self, acx, true)
    }
}

// TODO we can delete these handle wrappers
pub struct Handle<'a, T: 'a> {
    pub inner: jsapi::Handle<T>,
    _p: PhantomData<&'a T>,
}

pub type HandleVal<'a> = Handle<'a, Value>;

pub struct HandleMut<'a, T: 'a> {
    pub inner: jsapi::MutableHandle<T>,
    _p: PhantomData<&'a T>,
}
