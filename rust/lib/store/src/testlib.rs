use std::cell::RefCell;
use std::io;
use std::rc::Rc;

use bytebuffer::ByteBuffer;

use futures::future;
use futures::future::{FutureResult, Ok};

use alloc::Scoped;
use data::Range;
use traits::{Source, Sink, TdError};

/// A KvSink with only one supported key/value: the null one.
pub struct NullKeyDummyKvSink {
    ok: bool,
    buf: Option<Rc<RefCell<ByteBuffer>>>,
}

impl NullKeyDummyKvSink {
    pub fn new() -> Self {
        NullKeyDummyKvSink {
            ok: true,
            buf: None,
        }
    }

    fn check_key<K: Scoped<[u8]>>(&self, k: K) -> bool {
        self.ok && k.get().unwrap() == []
    }
}

impl Source<[u8]> for NullKeyDummyKvSink {
    type Get = Box<[u8]>;
    type GetF = Ok<Option<Self::Get>, TdError>;

    fn get<K: Scoped<[u8]>>(&mut self, k: K) -> Self::GetF {
        if self.check_key(k) {
            match self.buf {
                Some(ref buf) => {
                    let mut buf = buf.borrow_mut();
                    let len = buf.len();
                    buf.set_rpos(0);
                    let r = buf.read_bytes(len).into_boxed_slice();
                    future::ok(Some(r))
                },
                None => future::ok(None),
            }
        } else {
            future::ok(None)
        }
    }

    fn subtree<K: Scoped<[u8]>>(&mut self, k: K) -> Self {
        NullKeyDummyKvSink {
            ok: k.get().unwrap() == [],
            buf: None,
        }
    }

    fn subrange(&mut self, range: Range) -> Self {
        NullKeyDummyKvSink {
            ok: false,
            buf: None,
        }
    }
}

impl Sink<[u8]> for NullKeyDummyKvSink {
    type PutF = FutureResult<(), TdError>;

    fn max_value_size(&self) -> u64 {
        65536
    }

    fn put_small<K: Scoped<[u8]>, V: Scoped<[u8]>>(&mut self, k: K, v: V) -> Self::PutF {
        if self.check_key(k) {
            // TODO: check value length
            self.buf = Some(Rc::new(RefCell::new(ByteBuffer::from_bytes(v.get().unwrap()))));
            future::result(Ok(()))
        } else {
            // TODO: should be a kv-specific error
            future::result(Err(TdError::IoError(io::Error::new(io::ErrorKind::NotFound, "Key not supported"))))
        }
    }
}
