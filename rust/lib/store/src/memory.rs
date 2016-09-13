use std::cell::RefCell;
use std::collections::HashMap;
use std::mem;
use std::ptr;
use std::rc::Rc;

use futures;

use traits::*;

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
    target: Option<Rc<TreeNode>>,
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

#[allow(dead_code)]
struct SnapshotImpl {
    head: SnapshotPtr,
    parent_handle: Rc<RefCell<EphemeralStoreHead>>,
}

fn snapshot_impl(p: &SnapshotPtr, h: &Rc<RefCell<EphemeralStoreHead>>) -> SnapshotImpl {
    SnapshotImpl {
        head: p.clone(),
        parent_handle: h.clone(),
    }
}

#[allow(dead_code)]
struct SnapshotDatumPointer {
    //val: *TreeValue,
}

impl Datum for SnapshotDatumPointer {
    fn len(&self) -> u16 {
        panic!("Not implemented")
    }

    fn write_bytes<W: DataWrite>(&self, w: &mut W) -> W::R where Self: Sized {
        panic!("Not implemented")
    }
}

impl KvSource for SnapshotImpl {
    type D = SnapshotDatumPointer;
    type R = futures::Done<Self::D, Error>;

    #[allow(unused_variables)]
    fn read(&self, k: &Datum) -> Self::R {
        futures::done(Err(Error::other("not yet implemented")))
    }
}
struct SnapshotStorePointer {
    // TODO: unsafe cell in non-debug
    target: Rc<RefCell<EphemeralStoreHead>>,
}

struct SnapshotStoreImpl {
    // This is the same head used in SnapshotStorePointer.
    target: Rc<RefCell<EphemeralStoreHead>>,
}

impl SnapshotStore for SnapshotStoreImpl {
    type Snap = SnapshotImpl;

    fn open(&mut self) -> Counter {
        // TODO safety check
        // TODO ephemeral snapshots also
        let mut head = (*self.target).borrow_mut();
        let ticker = head.ticker.clone();
        let headptr = head.head.clone(); // Early clone to evade borrow rules
        head.kvs.insert(ticker.clone(), SnapshotPtr::new(&headptr));
        head.ticker = ticker.inc();
        ticker
    }

    fn close(&mut self, stamp: &Counter) {
        let mut head = (*self.target).borrow_mut();
        head.kvs.remove(stamp);
        ()
    }

    //fn diff(&self, &prev: SnapshotStamp) -> KvStream

    fn snap(&self, stamp: &Counter) -> Option<Self::Snap> {
        let head = (*self.target).borrow();
        match head.kvs.get(stamp) {
            Some(snaphead) => Some(snapshot_impl(&snaphead, &self.target)),
            None => None,
        }
    }
}

pub fn ephemeral_store() -> impl SnapshotStore {
    SnapshotStoreImpl {
        target: Rc::new(RefCell::new(EphemeralStoreHead::new()))
    }
}

