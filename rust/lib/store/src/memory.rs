use std::collections::HashMap;
use std::mem;
use std::ops::Deref;
use std::ptr;
// use std::rc::Rc;
use std::sync::{Arc, Mutex, MutexGuard};

use data::*;
use traits::*;

/*
Design of MVP kvs

We need two things here:
* ACID transactions (real ones),
* persistent snapshots and cursors.

---

ACID

The best kv backends do not seem to handle ACID
(LightningDB says they do, but they're lying) so we need
to do it ourselves. Here's the plan:

Each key should be associated with an ephemeral intent.
It has a transaction counter and an RW value. Once distributed
transactions exist, we will need durable write intents; that comes
later.

We can also associate ranges with ephemeral intents; needed
for indexing. This suggests the intent map should be a tree.

The intents help us figure out our transactions.

---

PERSISTENT SNAPSHOTS

Ok, this is hard. The following DBs do NOT do persistent snapshots:
* WiredTiger
* LevelDB
* RocksDB
* Berkeley
* Bitcask
* {To, kyo} cabinet

We might just have to implement our own data structure :(

Basically, hitch hiker trees.

But we should def make a test datastructure. How about:
* Tree: a dumb kv store with buckets...
actually see hitchhiker tree redis backend
*/

#[allow(dead_code)]
struct TreeValue {
    val: Vec<i8>
}

impl TreeValue {
}

#[allow(dead_code)]
struct TreeValuePtr {
    target: Option<TreeValue>
}

#[allow(dead_code)]
impl TreeValuePtr {
    fn empty() -> TreeValuePtr {
        TreeValuePtr {
            target: None,
        }
    }
}

#[allow(dead_code)]
struct TreeNode {
    vals: [TreeValuePtr; 16],
    children: [TreeNodePtr; 16],
    // TODO do we need refcount?
    refcount: i32,
}

// TODO move to private util rs
/// Make an array, populating each element according to a lambda of one int.
macro_rules! make_array {
    ($constructor: expr, $n: expr) => {
        {
            let mut items: [_; $n] = mem::uninitialized();
            for (i, place) in items.iter_mut().enumerate() {
                ptr::write(place, $constructor(i));
            }
            items
        }
    }
}

#[allow(dead_code)]
impl TreeNode {
    fn empty() -> TreeNode {
        unsafe {
            let r = TreeNode {
                vals: make_array!(|_| TreeValuePtr::empty(), 16),
                children: make_array!(|_| TreeNodePtr::empty(), 16),
                refcount: 0,
            };
            r
        }
    }
}

#[derive(Clone)]
struct TreeNodePtr {
    target: Option<Arc<TreeNode>>,
}

impl TreeNodePtr {
    fn empty() -> TreeNodePtr {
        TreeNodePtr {
            target: None,
        }
    }
}

#[derive(Clone)]
#[allow(dead_code)]
struct SnapshotPtr {
    target: TreeNodePtr,
    open: bool,
}

impl SnapshotPtr {
    fn new(p: &TreeNodePtr) -> SnapshotPtr {
        SnapshotPtr {
            target: p.clone(),
            open: true,
        }
    }
}

#[allow(dead_code)]
struct EphemeralStoreHead {
    // TODO deterministic hasher
    kvs: HashMap<Counter, SnapshotPtr>,
    head: TreeNodePtr,
    ticker: Counter,
    open: bool,
}

impl EphemeralStoreHead {
    fn new() -> EphemeralStoreHead {
        EphemeralStoreHead {
            kvs: HashMap::new(),
            head: TreeNodePtr::empty(),
            ticker: Counter::new(0),
            open: true,
        }
    }
}

#[derive(Clone)]
struct EphemeralStoreHandle {
    target: Arc<Mutex<EphemeralStoreHead>>,
}

impl EphemeralStoreHandle {
    fn new(h: EphemeralStoreHead) -> EphemeralStoreHandle {
        EphemeralStoreHandle { target: Arc::new(Mutex::new(h)) }
    }

    // fn apply<T>(f: EphemeralStoreHead -> T) -> T {
    //     let head = (*target).lock().unwrap()
    //     f(head)
    // }

    fn lock<'a>(&'a self) -> MutexGuard<'a, EphemeralStoreHead> {
        // TODO: handle panic here
        self.target.deref().lock().unwrap()
    }
}

#[allow(dead_code)]
struct SnapshotImpl {
    head: SnapshotPtr,
    parent_handle: EphemeralStoreHandle,
}

impl SnapshotImpl {
    fn new(p: &SnapshotPtr, h: &EphemeralStoreHandle) -> SnapshotImpl {
        SnapshotImpl {
            head: p.clone(),
            parent_handle: h.clone(),
        }
    }
}

impl KvSource for SnapshotImpl {
    type D = SnapshotDatumPointer;
    type R = Done<Self::D>;

    #[allow(unused_variables)]
    fn read<DR: Datum>(&self, k: &DR) -> Self::R {
        err(Error::other("not yet implemented"))
    }
}

#[allow(dead_code)]
struct SnapshotImplMut {
    parent_handle: EphemeralStoreHandle,
    // TODO: consistent naming--ticker or counter
    ticker: Counter,
}

impl SnapshotImplMut {
    fn new(h: &EphemeralStoreHandle, c: Counter) -> SnapshotImplMut {
        SnapshotImplMut {
            parent_handle: h.clone(),
            ticker: c,
        }
    }
}

impl KvSource for SnapshotImplMut {
    type D = SnapshotDatumPointer;
    type R = Done<Self::D>;

    #[allow(unused_variables)]
    fn read<DR: Datum>(&self, k: &DR) -> Self::R {
        err(Error::other("not yet implemented"))
    }
}

impl KvSink for SnapshotImplMut {
    type R = Done<()>;

    #[allow(unused_variables)]
    fn write<D1: Datum, D2: Datum>(&mut self, k: &D1, v: &D2) -> Self::R {
        err(Error::other("not yet implemented"))
    }
}

#[allow(dead_code)]
struct SnapshotDatumPointer {
    //val: *TreeValue,
}

#[allow(dead_code, unused_variables)]
impl Datum for SnapshotDatumPointer {
    fn len(&self) -> u16 {
        panic!("Not implemented")
    }

    fn write_bytes<W: DataWrite>(&self, w: W) -> W::Result where Self: Sized {
        panic!("Not implemented")
    }
}

struct SnapshotStoreImpl {
    target: EphemeralStoreHandle,
}

impl SnapshotStore for SnapshotStoreImpl {
    type Snap = SnapshotImpl;
    type SnapTmp = SnapshotImpl; // TODO a different type
    type SnapMut = SnapshotImplMut;

    type SnapF = Done<Self::Snap>;
    fn snap(&self, stamp: &Counter) -> Self::SnapF {
        let ref head = *self.target.lock();
        match head.kvs.get(stamp) {
            Some(snaphead) => ok(SnapshotImpl::new(&snaphead, &self.target)),
            None => err(Error::other("snapshot does not exist")), // TODO notfound
        }
    }

    // TODO: RO snapshots shouldn't increment the counter.
    // We're confusing snapshot fxnality with cursor fxnality here.
    type SnapNewF = Done<Self::Snap>;
    fn snap_new(&mut self) -> Self::SnapNewF {
        let ref mut head = *self.target.lock();
        let newsnap = SnapshotPtr::new(&head.head.clone());
        head.kvs.insert(head.ticker, newsnap.clone());
        ok(SnapshotImpl::new(&newsnap, &self.target))
    }

    type SnapTmpF = Done<Self::SnapTmp>;
    fn snap_tmp(&mut self) -> Self::SnapTmp {
        panic!("Not yet implemented")
    }

    type SnapMutF = Done<Self::SnapMut>;
    fn snap_mut(&mut self) -> Self::SnapMutF {
        let ref head = *self.target.lock();
        let newsnap = SnapshotImplMut::new(&self.target, head.ticker);
        ok(newsnap)
    }

    type SnapCloseF = Done<()>;
    fn snap_close(&mut self, stamp: &Counter) -> Self::SnapCloseF {
        let ref mut head = *self.target.lock();
        head.kvs.remove(stamp);
        // TODO close the snapshot
        ok(())
    }

    type CloseF = Done<()>;
    fn close(&mut self) -> Self::CloseF {
        let ref mut head = *self.target.lock();
        head.open = false;
        ok(())
    }

    //fn diff(&self, &prev: SnapshotStamp) -> KvStream
}

pub fn ephemeral_store() -> impl SnapshotStore {
    SnapshotStoreImpl {
        target: EphemeralStoreHandle::new(EphemeralStoreHead::new())
    }
}
