use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use js::jsapi::{self, JSContext, JSObject, JSScript, Value};
use js::rust;
use libc::size_t;

use thunderhead_store::TdError;

use engine::value::NativeValue;

use super::active_context::{self, ActiveContext, ActiveContextInner};

pub struct Rooted<T> {
    // All jsapi types are pass-by-copy and opaque to Rust, so we don't need to worry about aliasing.
    // Otherwise we would wrap this in an UnsafeCell.
    inner: jsapi::Rooted<T>,
}

pub type RootedObj = Rooted<*mut JSObject>;
pub type RootedScript = Rooted<*mut JSScript>;
pub type RootedVal = Rooted<Value>;

impl<T> Rooted<T> where T: Clone {
    pub fn new(t: T, cx: &mut JSContext) -> Self where T: rust::RootKind {
        let mut inner = jsapi::Rooted::new_unrooted(t);
        unsafe {
            // TODO: how does rooting work in rust?
            inner.add_to_root_stack(cx);
        }

        Rooted {
            inner: inner,
        }
    }

    pub fn get(&self) -> T {
        self.inner.ptr.clone()
    }

    fn get_ref(&self) -> &T {
        &self.inner.ptr
    }

    fn get_ref_mut(&mut self) -> &mut T {
        &mut self.inner.ptr
    }

    pub fn handle<'a>(&'a self) -> Handle<'a, T> {
        Handle {
            inner: unsafe { jsapi::Handle::from_marked_location(self.get_ref()) },
            _p: PhantomData,
        }
    }

    pub fn handle_mut<'a>(&'a mut self) -> HandleMut<'a, T> {
        HandleMut {
            inner: unsafe { jsapi::MutableHandle::from_marked_location(self.get_ref_mut()) },
            _p: PhantomData,
        }
    }
}

impl RootedVal {
    pub fn to_string(&self, acx: &mut ActiveContextInner, force: bool) -> Result<String, TdError> {
        unsafe {
            let mut jcx = acx.js_context();

            if self.get().is_string() || force {
                let js_str = if force {
                    rust::ToString(&mut *jcx, self.handle().inner)
                } else {
                    self.get().to_string()
                };

                // TODO: write to buffer instead. this is dumb.
                let mut buf = [0 as u8; 65536];
                let v = jsapi::JS_EncodeStringToBuffer(&mut *jcx, js_str, &mut buf[0] as *mut u8 as *mut i8, 65536);

                if v > 65536 {
                    // TODO shouldn't happen
                    Err(TdError::RuntimeError("String too big".into()))
                } else if v == (0 as size_t).wrapping_sub(1) {
                    Err(TdError::RuntimeError("Could not encode string".into()))
                } else {
                    Ok(String::from_utf8_lossy(&buf[..v]).into_owned())
                }
            } else {
                Err(TdError::RuntimeError("Failed string cast with force == false".into()))
            }
        }
    }

    pub fn to_native_value(&mut self, acx: &mut ActiveContext) -> Result<NativeValue, TdError> {
        let v = self.get();

        let r = if v.is_null() {
            NativeValue::Null
        } else if v.is_boolean() {
            NativeValue::Bool(v.to_boolean())
        } else if v.is_double() {
            NativeValue::Double(v.to_double())
        } else if v.is_int32() {
            NativeValue::Int(v.to_int32())
        } else if v.is_string() {
            NativeValue::String(self.to_string(active_context::inner(acx), false).unwrap())
        } else if v.is_object() {
            panic!()
        } else if v.is_undefined() {
            return Err(TdError::RuntimeError("Inconvertible value: undefined".into()));
        } else if v.is_symbol() {
            return
            Err(TdError::RuntimeError(self.to_string(active_context::inner(acx), true)
            .map(|s| format!("Inconvertible value: {}", s))
            .unwrap_or("Inconvertible value. Additionally, could not convert value to string".into())));
        } else {
            unreachable!()
        };

        Ok(r)
    }

    // fn is_function(&self, acx: &mut ActiveContext) -> bool {
    //     if self.get().is_object() {
    //         unsafe {
    //             jsapi::JS_ObjectIsFunction(&mut *active_context::inner(acx).js_context(), self.get().to_object())
    //         }
    //     } else {
    //         false
    //     }
    // }

    // fn debug_string(&mut self, acx: &mut ActiveContext) -> Result<String, TdError> {
    //     // TODO: shorter strings
    //     self.to_string(active_context::inner(acx), true)
    // }
    //
    // // TODO: write to bytes
    // fn serialize(&mut self, acx: &mut ActiveContext) -> Result<String, TdError> {
    //     // TODO: shorter strings
    //     self.to_string(active_context::inner(acx), true)
    // }
}

pub struct Handle<'a, T: 'a> {
    /// This is unique: no two Handles should ever have the same inner.
    inner: jsapi::Handle<T>,
    _p: PhantomData<&'a T>,
}

impl<'a, T> Deref for Handle<'a, T> {
    type Target = jsapi::Handle<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub type HandleVal<'a> = Handle<'a, Value>;

pub struct HandleMut<'a, T: 'a> {
    inner: jsapi::MutableHandle<T>,
    _p: PhantomData<&'a T>,
}

impl<'a, T> Deref for HandleMut<'a, T> {
    type Target = jsapi::MutableHandle<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, T> DerefMut for HandleMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
