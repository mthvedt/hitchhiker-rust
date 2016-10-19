use std::borrow::Borrow;
use std::mem;

use data::{ByteRc, ByteWeak, Datum};

use tree::counter::*;

#[derive(Clone)]
// TODO ByteRc -> RcBytes
pub struct Bucket {
    k: ByteRc,
    v: ByteRc,
}

impl Bucket {
    fn downgrade(&self) -> WeakBucket {
        WeakBucket {
            k: self.k.downgrade(),
            v: self.v.downgrade(),
        }
    }
}

// TODO: maybe (k, v) and (k, PersistentValue)? How to present a clean interface?
pub enum BucketRef {
    Transient(Bucket),
    Persistent(Bucket, Counter),
}

impl BucketRef {
    pub fn new_transient<V: Datum>(k: &[u8], v: &V) -> BucketRef {
        BucketRef::Transient(Bucket {
            k: ByteRc::from_key(k),
            v: ByteRc::from_value(v),
        })
    }

    pub fn key(&self) -> &[u8] {
        match *self {
            BucketRef::Transient(ref b) => b.k.borrow(),
            BucketRef::Persistent(ref b, _) => b.k.borrow(),
        }
    }

    pub fn value(&self) -> &ByteRc {
        match *self {
            BucketRef::Transient(ref b) => &b.v,
            BucketRef::Persistent(ref b, _) => &b.v,
        }
    }

    pub fn txid(&self) -> Counter {
        match *self {
            BucketRef::Transient(_) => panic!("Can't call txid on a transient Bucket"),
            BucketRef::Persistent(_, txid) => txid,
        }
    }

    /// Makes this BucketRef immutable, if it wasn't already.
    pub fn immute(&mut self, txid: Counter) {
        // TODO: use mem::replace
        let mut oldself = unsafe { mem::uninitialized() };
        let mut newself;
        // Now self is uninit
        mem::swap(self, &mut oldself);

        // Can't figure out how to do this more elegantly... hopefully it will optimize
        match oldself {
            BucketRef::Transient(b) => {
                newself = BucketRef::Persistent(b, txid);
            }
            BucketRef::Persistent(b, txid) => {
                newself = BucketRef::Persistent(b, txid);
            }
        }

        // Now newself is uninit
        mem::swap(&mut newself, self);
        mem::forget(newself);
    }

    pub fn shallow_clone(&self) -> BucketRef {
        match *self {
            BucketRef::Transient(_) => panic!("Can't clone a transient Bucket"),
            BucketRef::Persistent(ref b, txid) => BucketRef::Persistent(b.clone(), txid),
        }
    }

    pub fn downgrade(&self) -> WeakBucketRef {
        match *self {
            BucketRef::Transient(ref b) => WeakBucketRef::Transient(b.downgrade()),
            BucketRef::Persistent(ref b, txid) => WeakBucketRef::Persistent(b.downgrade(), txid),
        }
    }
}

#[derive(Clone)]
// Todo ByteWeak -> WeakBytes
/// Public for API purposes.
pub struct WeakBucket {
    k: ByteWeak,
    v: ByteWeak,
}

pub enum WeakBucketRef {
    Transient(WeakBucket),
    Persistent(WeakBucket, Counter),
}

impl WeakBucketRef {
    pub fn key(&self) -> ByteRc {
        match *self {
            WeakBucketRef::Transient(ref b) => b.k.upgrade(),
            WeakBucketRef::Persistent(ref b, _) => b.k.upgrade(),
        }
    }

    pub fn value(&self) -> ByteRc {
        match *self {
            WeakBucketRef::Transient(ref b) => b.v.upgrade(),
            WeakBucketRef::Persistent(ref b, _) => b.v.upgrade(),
        }
    }

    pub fn txid(&self) -> Counter {
        match *self {
            WeakBucketRef::Transient(_) => panic!("Can't call txid on a transient Bucket"),
            WeakBucketRef::Persistent(_, txid) => txid,
        }
    }
}
