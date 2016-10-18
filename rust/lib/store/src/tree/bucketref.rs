use std::borrow::Borrow;
use std::mem;

use data::{ByteRc, Datum};

use tree::counter::*;

#[derive(Clone)]
pub struct Bucket {
    k: ByteRc,
    v: ByteRc,
}

impl Bucket {
    fn new<V: Datum>(k: &[u8], v: &V) -> Bucket {
        Bucket {
            // TODO: from_key or from_bytes?
            k: ByteRc::from_key(k),
            v: ByteRc::from_value(v),
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
        BucketRef::Transient(Bucket::new(k, v))
    }

    pub fn key(&self) -> &[u8] {
        match self {
            &BucketRef::Transient(ref b) => b.k.borrow(),
            &BucketRef::Persistent(ref b, _) => b.k.borrow(),
        }
    }

    pub fn value(&self) -> &ByteRc {
        match self {
            &BucketRef::Transient(ref b) => &b.v,
            &BucketRef::Persistent(ref b, _) => &b.v,
        }
    }

    pub fn txid(&self) -> Counter {
        match self {
            &BucketRef::Transient(_) => panic!("Can't call txid on a transient Bucket"),
            &BucketRef::Persistent(_, txid) => txid,
        }
    }

    /// Makes this BucketRef immutable, if it wasn't already.
    pub fn immute(&mut self, txid: Counter) {
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
        match self {
            &BucketRef::Transient(_) => panic!("Can't clone a transient Bucket"),
            &BucketRef::Persistent(ref b, txid) => BucketRef::Persistent(b.clone(), txid),
        }
    }
}
