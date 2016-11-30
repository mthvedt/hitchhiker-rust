use std::cell::RefCell;
use std::marker::PhantomData;
use std::mem;
use std::rc::{Rc, Weak};

use futures::{Future, Poll};

use thunderhead_store::{KvSource, KvSink, TdError};
use thunderhead_store::alloc::Scoped;
use thunderhead_store::tdfuture::FutureMap;

use lens::{Lens, StringLens};
use engine::js::Processor;

// PLAN FOR JS:
//
// KV--too heavyweight and difficult. Also, we usually read whole JSON documents.
// So... flat files it is.

// Design: A 'wire' type, a 'binary' type, and a 'JSON' type, and lenses for each.
// Some kinda combined lens can yield both.

// /// A Json-wrapping type. It's intentionally opaque, since its inner fields are not intended for Rust.
// /// (Not yet anyway.)
// struct TdJson {
//
// }

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

// /// A lens that turns REST wire-format JSON into Spidermonkey JS.
// struct SmTextJsonLens;
//
// /// A lens that turns REST JSON into JSON blobs.
// struct JsonTextDataLens;

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

pub enum ErrorPropogator<F: Future> {
    Ok(F),
    Err(F::Error),
    Done,
}

impl<F: Future> Future for ErrorPropogator<F> {
    type Item = F::Item;
    type Error = F::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let &mut ErrorPropogator::Ok(ref mut f) = self {
            return f.poll();
        }

        match mem::replace(self, ErrorPropogator::Done) {
            ErrorPropogator::Ok(_) => unreachable!(),
            ErrorPropogator::Err(e) => Err(e),
            ErrorPropogator::Done => panic!("cannot poll a completed future twice"),
        }
    }
}

#[derive(Clone)]
struct JsToTextProcessorLens {
    // read: Rc<RefCell<Processor>>,
    write: Rc<RefCell<Processor>>,
}

impl JsToTextProcessorLens {
    fn new(write_processor: Processor) -> Self {
        JsToTextProcessorLens {
            write: Rc::new(RefCell::new(write_processor)),
        }
    }
}

impl<S: KvSource + KvSink> Lens<S> for JsToTextProcessorLens {
    type Target = String;

    // type ReadResult = MapFuture<S::GetF, ProcessorRead<S::Get, S::GetF>>;
    //
    // fn read(&self, source: &mut S) -> Self::ReadResult {
    //     let pr = ProcessorRead {
    //         inner: Rc::downgrade(&self.read),
    //         _p: PhantomData,
    //     };
    //
    //     MapFuture::new(source.get([]), pr)
    // }

    // TODO: this is slow
    type ReadResult = <StringLens as Lens<S>>::ReadResult;

    fn read(&self, source: &mut S) -> Self::ReadResult {
        // TODO: debug verify javascript?
        StringLens.read(source)
    }

    type WriteResult = ErrorPropogator<S::PutF>;

    fn write<V: Scoped<Self::Target>>(&self, target: V, sink: &mut S) -> Self::WriteResult {
        let mut pxrref = self.write.borrow_mut();
        let result = pxrref.apply(target.get().unwrap().as_ref());
        let result_str = result.and_then(|result| pxrref.to_string(result));

        match result_str {
            Ok(result_str) => ErrorPropogator::Ok(StringLens.write(result_str, sink)),
            Err(e) => ErrorPropogator::Err(e),
        }
    }
}

#[cfg(test)]
mod test {
    use super::JsToTextProcessorLens;

    use futures::Future;

    use thunderhead_store::testlib::NullKeyDummyKvSink;

    use engine::js::{Processor, RuntimeHandle};
    use lens::Lens;
    use system::SystemScripts;

    #[test]
    fn test_json_processor() {
        let mut r = RuntimeHandle::new_runtime();
        let pxr = Processor::from_source(r.new_environment(), "js/serialize_json", SystemScripts).wait().ok().unwrap();

        let lens = JsToTextProcessorLens::new(pxr);

        let mut s = NullKeyDummyKvSink::new();

        lens.write(String::from("{\"x\": 1}"), &mut s).wait().ok();
        let r = lens.read(&mut s).wait().ok().unwrap().unwrap();
        assert!(r == "{\"x\":1}");
    }
}
