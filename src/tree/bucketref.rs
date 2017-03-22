use std::borrow::Borrow;
use std::mem;

use data::{RcBytes, WeakBytes, Datum};

use counter::Counter;

#[derive(Clone)]
// TODO RcBytes -> RcBytes
/// Public so we do not have a private type in a public interface.
pub struct Bucket {
    k: RcBytes,
    v: RcBytes,
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
            k: RcBytes::new(k),
            v: RcBytes::from_value(v),
        })
    }

    // TODO: get rid of Datum, make this transient
    pub fn transient_from_bytes(k: &[u8], v: &[u8]) -> BucketRef {
        BucketRef::Transient(Bucket {
            k: RcBytes::new(k),
            v: RcBytes::new(v),
        })
    }

    pub fn key(&self) -> &[u8] {
        match *self {
            BucketRef::Transient(ref b) => b.k.borrow(),
            BucketRef::Persistent(ref b, _) => b.k.borrow(),
        }
    }

    pub fn value(&self) -> &RcBytes {
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
        let mut newself = match *self {
            BucketRef::Transient(ref b) => BucketRef::Persistent(b.clone(), txid),
            BucketRef::Persistent(ref b, old_txid) => BucketRef::Persistent(b.clone(), old_txid),
        };

        // Now newself is uninit
        mem::swap(&mut newself, self);
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
/// Public for API purposes.
pub struct WeakBucket {
    k: WeakBytes,
    v: WeakBytes,
}

pub enum WeakBucketRef {
    Transient(WeakBucket),
    Persistent(WeakBucket, Counter),
}

impl WeakBucketRef {
    pub fn key(&self) -> RcBytes {
        match *self {
            WeakBucketRef::Transient(ref b) => b.k.upgrade(),
            WeakBucketRef::Persistent(ref b, _) => b.k.upgrade(),
        }
    }

    pub fn value(&self) -> RcBytes {
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
