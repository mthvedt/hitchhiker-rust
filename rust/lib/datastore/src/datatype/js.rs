use futures::Future;

use thunderhead_store::{KvSource, KvSink, TdError};
use thunderhead_store::alloc::Scoped;
use thunderhead_store::tdfuture::{BoxFuture, FutureExt};

use engine::{self, spidermonkey};
use lens::{ReadLens, StringLens, WriteLens};

type ProcessorHandle = engine::ProcessorHandle<spidermonkey::Spec>;

/// A (read, write) lens, wrapping a Processor that maps JSON input to JSON output.
/// Note that this is NOT a bidirectional lens; the read value is simply the identity.
#[derive(Clone)]
pub struct JsToTextProcessorLens {
    write: ProcessorHandle,
}

impl JsToTextProcessorLens {
    pub fn new(write_processor: ProcessorHandle) -> Self {
        JsToTextProcessorLens {
            write: write_processor,
        }
    }
}

impl<S: KvSource> ReadLens<S> for JsToTextProcessorLens {
    type Target = String;

    // TODO: this is slow
    type ReadResult = <StringLens as ReadLens<S>>::ReadResult;

    fn read(&self, source: S) -> Self::ReadResult {
        // TODO: debug verify javascript?
        StringLens.read(source)
    }
}

impl<S: KvSink + 'static> WriteLens<S> for JsToTextProcessorLens {
    type Target = String;

    type WriteResult = BoxFuture<(), TdError>;

    fn write<V: Scoped<Self::Target>>(&self, target: V, sink: S) -> Self::WriteResult {
        // TODO: can we assert output is json?
        self.write.apply_and_write(target.get().unwrap().as_ref()).and_then(|(_, rs)| {
            StringLens.write(rs, sink)
        }).td_boxed()
    }
}

#[cfg(test)]
mod test {
    use futures::Future;

    use thunderhead_store::testlib::NullKeyDummyKvSink;

    use engine::{Engine, EngineSpec, Factory, FactoryHandle, ProcessorHandle};
    use engine::spidermonkey::Spec;
    use lens::{ReadLens, WriteLens};
    use system::SystemScripts;

    use super::JsToTextProcessorLens;

    #[test]
    fn test_json_processor() {
        let f = Spec::new_factory().unwrap();
        let cx = f.handle().new_engine().unwrap().new_context().unwrap();
        let pxr = ProcessorHandle::processor_from_source(cx, "js/serialize_json", SystemScripts).wait().unwrap();

        let lens = JsToTextProcessorLens::new(pxr);

        let s = NullKeyDummyKvSink::new();

        lens.write(String::from("{\"x\": 1}"), s.clone()).wait().ok();
        let r = lens.read(s.clone()).wait().ok().unwrap().unwrap();
        assert!(r == "{\"x\":1}");
    }
}
