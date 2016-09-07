use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;

use data::*;
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

#[allow(dead_code)]
struct TreeNode {
    vals: [TreeValuePtr; 16],
    children: [TreeNodePtr; 16],
    refcount: i32,
}

impl TreeNode {
}

#[derive(Clone)]
struct TreeNodePtr {
    // TODO: instead Rc?
    target: Option<Rc<TreeNode>>,
}

impl TreeNodePtr {
    fn unwrap<'a>(&'a self) -> Option<&'a TreeNode> {
        match self.target {
            Some(ref r) => Some(&(*r)),
            None => None,
        }
    }
}

#[allow(dead_code)]
struct SnapshotImpl<'a> {
    head: Option<&'a TreeNode>,
}

impl<'a> KvSource for SnapshotImpl<'a> {
    type D = SliceDatum<'a>;

    #[allow(unused_variables)]
    fn read(&self, k: &Datum) -> KvResult<Self::D> {
        KvResult::Failure(String::from("not yet implemented"))
    }
}

#[allow(dead_code)]
struct SnapshotStoreImpl<'a> {
    // TODO deterministic hasher
    kvs: HashMap<SnapshotStampImpl, TreeNodePtr>,
    head: TreeNodePtr,
    ticker: Counter,
    phantom: PhantomData<&'a ()>
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct SnapshotStampImpl {
    datum: Counter
}

impl SnapshotStamp for SnapshotStampImpl {
}

impl<'a> SnapshotStore<'a> for SnapshotStoreImpl<'a> {
    type S = SnapshotImpl<'a>;
    type Stamp = SnapshotStampImpl;

    fn open(&mut self) -> SnapshotStampImpl {
        // TODO safety check
        // TODO ephemeral snapshots also
        let stamp = SnapshotStampImpl {
            datum: self.ticker.clone(),
        };
        self.kvs.insert(stamp.clone(), self.head.clone());
        self.ticker = self.ticker.inc();
        stamp
    }

    fn close(&mut self, stamp: &SnapshotStampImpl) {
        self.kvs.remove(stamp);
        ()
    }

    //fn diff(&self, &prev: SnapshotStamp) -> KvStream

    fn snap(&'a self, stamp: &'a SnapshotStampImpl) -> Option<Self::S> {
        match self.kvs.get(stamp) {
            Some(snaphead) => Some(SnapshotImpl { head: snaphead.unwrap() }),
            None => None,
        }
    }
}

pub fn ephemeral_store<'a>() -> impl SnapshotStore<'a> {
    SnapshotStoreImpl {
        kvs: HashMap::new(),
        head: TreeNodePtr { target: None },
        ticker: Counter::new(0),
        phantom: PhantomData,
    }
}

