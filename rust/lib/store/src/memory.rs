use std::collections::HashMap::hash_map

struct TreeValue {
    val: Vec<i8>
}

impl TreeValue {
}

struct TreeValuePtr {
    target: Optional<TreeValue>
}

struct TreeNode {
    // TODO define tree size
    vals: [TreeValuePtr; 16]
    children: [TreeNodePtr; 16]
    refcount: i32
}

impl TreeNode {
}

struct TreeNodePtr {
    target: Optional<Arc<TreeNode>>
}

impl TreeNodePtr {
}

struct SnapshotImpl {
    // TODO lifetime this
    head: TreeNodePtr
}

impl KvSource for SnapshotImpl {
    read(&self, &k: Datum) -> KvResult<Datum> {
    }
}

struct SnapshotStoreImpl {
    // TODO deterministic hasher
    kvs: HashMap<SnapshotStampByteString, TreeNodePtr>
    head: TreeNodePtr
    ticker: i32
}

impl SnapshotStore for SnapshotStoreImpl {
    fn open(&mut self) -> SnapshotStamp {
        // TODO safety check
        // TODO ephemeral snapshots also
        let stamp = SnapshotStampByteString(ticker)
        kvs.insert(stamp, head.clone())
        ticker += 1
        stamp
    }

    fn close(&mut self, &stamp: SnapshotStamp) {
        kvs.remove(stamp)
    }

    //fn diff(&self, &prev: SnapshotStamp) -> KvStream

    fn snap(&self, &stamp: SnapshotStamp) -> Optional<KvSource> {
        let snaphead = kvs.get(stamp)
        SnapshotImpl(snaphead)
    }
}

