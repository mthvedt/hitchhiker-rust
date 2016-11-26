use std::marker::PhantomData;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

use futures::Future;

use thunderhead_store::{KvSource, KvSink, TdError};
use thunderhead_store::alloc::Scoped;
use thunderhead_store::tdfuture::{FutureMap, MapFuture};

use datatype::io::Lens;
use engine::js::Processor;

// PLAN FOR JS:
//
// KV--too heavyweight and difficult. Also, we usually read whole JSON documents.
// So... flat files it is.

// Design: A 'wire' type, a 'binary' type, and a 'JSON' type, and lenses for each.
// Some kinda combined lens can yield both.

/// A Json-wrapping type. It's intentionally opaque, since its inner fields are not intended for Rust.
/// (Not yet anyway.)
struct TdJson {

}

// This is not yet supported. Instead we use SpiderMonkey directly.
// /// A lens for text JSON, which is what TD currently sends/receives over the wire.
// struct RestJsonLens;

// impl<S: KvSink> Lens<S> for TextJsonLens {
//     type Target = String;

//     type ReadResult: Future<Item = Self::Target, Error = io::Error>;

//     fn read(&self, source: &mut S) -> FutureResult<Self::ReadResult> {

//     }

//     type WriteResult: Future<Item = (), Error = io::Error>;

//     fn write<V: Scoped<Self::Target>>(&self, target: V, sink: &mut S) -> FutureResult<Self::WriteResult> {

//     }
// }

/// A lens that turns binary JSON blobs into Spidermonkey JS.
struct JsonIntakeLens;

/// A lens that turns REST wire-format JSON into Spidermonkey JS.
struct SmTextJsonLens;

/// A lens that turns REST JSON into JSON blobs.
struct JsonTextDataLens;

pub struct ProcessorRead<I, GetF> {
    inner: Weak<RefCell<Processor>>,
    _p: PhantomData<(*const I, *const GetF)>,
}

impl<I: Scoped<[u8]>, GetF: Future<Item = Option<I>>> FutureMap for ProcessorRead<I, GetF> {
    type Input = GetF::Item;
    type Output = Option<String>;
    type Error = TdError;

    fn apply(&mut self, iopt: Self::Input) -> Result<Self::Output, TdError> {
        match iopt {
            Some(i) => {
                let rc_pxr = self.inner.upgrade().unwrap();
                let vr = rc_pxr.borrow_mut().apply(i);
                vr.and_then(|v| rc_pxr.borrow_mut().to_string(v).map(|x| Some(x)))
            },
            None => Ok(None),
        }
    }
}

#[derive(Clone)]
struct JsToTextProcessorLens {
    read: Rc<RefCell<Processor>>,
    write: Rc<RefCell<Processor>>,
}

impl<S: KvSource + KvSink> Lens<S> for JsToTextProcessorLens {
    type Target = String;

    type ReadResult = MapFuture<S::GetF, ProcessorRead<S::Get, S::GetF>>;

    fn read(&self, source: &mut S) -> Self::ReadResult {
        let pr = ProcessorRead {
            inner: Rc::downgrade(&self.read),
            _p: PhantomData,
        };

        MapFuture::new(source.get([]), pr)
    }

    type WriteResult = S::PutF;

    fn write<V: Scoped<Self::Target>>(&self, target: V, sink: &mut S) -> Self::WriteResult {
        panic!("TODO")
    }
}

#[cfg(test)]
mod test {

}
