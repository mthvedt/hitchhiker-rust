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
    buf: Option<Rc<RefCell<Option<ByteBuffer>>>>,
}

impl NullKeyDummyKvSink {
    pub fn new() -> Self {
        NullKeyDummyKvSink {
            ok: true,
            buf: Some(Rc::new(RefCell::new(None))),
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
            future::ok(self.buf.as_mut().and_then(|rc| rc.borrow_mut().as_mut().and_then(|mut buf| {
                let len = buf.len();
                buf.set_rpos(0);
                let r = buf.read_bytes(len).into_boxed_slice();
                Some(r)
            })))
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
            match self.buf {
                Some(ref rc_buf) => {
                    *rc_buf.borrow_mut() = Some(ByteBuffer::from_bytes(v.get().unwrap()));
                    future::result(Ok(()))
                },
                None => future::result(Err(TdError::IoError(
                    io::Error::new(io::ErrorKind::NotFound, "Key not supported")))),
            }
        } else {
            // TODO: should be a kv-specific error, not an io error
            future::result(Err(TdError::IoError(io::Error::new(io::ErrorKind::NotFound, "Key not supported"))))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::NullKeyDummyKvSink;

    use futures::Future;

    use traits::{Source, Sink};

    // TODO: test subrange/subtree
    #[test]
    fn test_null_key_dummy_kv_sink() {
        let mut s = NullKeyDummyKvSink::new();

        s.put_small([], "asdf".as_ref()).wait().ok().unwrap();
        let r = s.get([]).wait().ok().unwrap().unwrap();
        assert!(String::from_utf8_lossy(&*r).into_owned() == "asdf");
        let r = s.get([]).wait().ok().unwrap().unwrap();
        assert!(String::from_utf8_lossy(&*r).into_owned() == "asdf");

        s.put_small([], "ghjk".as_ref()).wait().ok().unwrap();
        let r = s.get([]).wait().ok().unwrap().unwrap();
        assert!(String::from_utf8_lossy(&*r).into_owned() == "ghjk");
        let r = s.get([]).wait().ok().unwrap().unwrap();
        assert!(String::from_utf8_lossy(&*r).into_owned() == "ghjk");
    }
}
