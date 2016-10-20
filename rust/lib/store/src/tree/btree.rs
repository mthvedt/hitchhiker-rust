use std::borrow::Borrow;

use data::*;

use tree::bucketref::*;
use tree::counter::*;
use tree::memnode::*;
use tree::noderef::*;

// TODO: move to module level doc the below.
// A key is anything that can be (quickly, efficiently) converted to raw bytes.
// A value is a Datum, a set of bytes that can be streamed.

/// A map that maps byte keys to data streams.
pub trait ByteMap {
	// TODO: these should be data streams.
	/// The type of a data stream.
	type GetDatum: Datum;

	/// The type returned by the get method.
	type Get: Borrow<Self::GetDatum>;

	/// This is mutable because gets may introduce read conflicts, and hence mutate the underlying datastructure.
	// TODO this does not need to be mutable. In particular, interior mutability is a thing
	fn get<K: Key + ?Sized>(&mut self, k: &K) -> Option<Self::Get>;

	/// Debug method to check this data structures's invariants.
	// TODO: isolate.
	fn check_invariants(&self);
}

/// A map also supporting insert and delete.
pub trait MutableByteMap: ByteMap {
	fn insert<K: Key + ?Sized, V: Datum>(&mut self, k: &K, v: &V) -> ();

	fn delete<K: Key + ?Sized>(&mut self, k: &K) -> bool;
}

pub trait NavigableByteMap: ByteMap {
	type Cursor: Cursor;

	fn cursor(&self, k: &[u8]) -> Self::Cursor;

	fn start_cursor(&self) -> Self::Cursor {
		self.cursor(&[])
	}
}

/// A snapshot map. Each snapshot has a transaction counter.
pub trait ByteMapSnapshot: NavigableByteMap {
	type Diff: NavigableByteMap;

	/// This snapshot's transaction counter.
	fn txid(&self) -> Counter;

	/// Gets a difference snapshot from this snapshot, containing all data in this snapshot
	/// committed after the given transaction counter.
	fn diff(&self, past: Counter) -> Self::Diff;
}

/// A byte map supporting snapshots.
// TODO name
pub trait FunctionalByteMap: MutableByteMap {
	type Snap: ByteMapSnapshot;

	fn snap(&mut self) -> Self::Snap;
}

pub trait Cursor {
	// TODO: these should be data streams.
	/// The type of a data stream.
	type GetDatum: Datum;

	/// The type returned by the get method.
	type Get: Borrow<Self::GetDatum>;

	fn key(&self) -> Option<ByteRc>;

	fn value(&self) -> Option<Self::Get>;

	fn advance(&mut self) -> bool;
}

// TODO: this does not need to be a mod
mod nodestack {
	use tree::bucketref::*;
	use tree::noderef::NodeRef;
	use tree::memnode::*;

	const MAX_DEPTH: u8 = 32;

	pub type NodeCursor = (NodeRef, u16);

	pub struct NodeStack {
		entries: Vec<NodeCursor>,
	}

	impl NodeStack {
		pub fn empty() -> NodeStack {
			NodeStack {
				// master_node: topnode,
				entries: Vec::with_capacity(MAX_DEPTH as usize),
			}
		}

		pub fn push(&mut self, node: NodeRef, child_index: u16) {
			debug_assert!(self.entries.len() < MAX_DEPTH as usize);
			self.entries.push((node, child_index));
		}

		pub fn pop(&mut self) -> Option<NodeCursor> {
			self.entries.pop()
		}

		pub fn peek(&self) -> Option<&NodeCursor> {
			if self.is_empty() {
				None
			} else {
				Some(&self.entries[self.entries.len() - 1])
			}
		}

		/// Peeks mutably at the top of the stack. Useful because pop-and-push optimizes poorly.
		pub fn peek_mut(&mut self) -> Option<&mut NodeCursor> {
			if self.is_empty() {
				None
			} else {
				let idx = self.entries.len() - 1; // borrow check
				Some(&mut self.entries[idx])
			}
		}

		pub fn is_empty(&self) -> bool {
			self.entries.is_empty()
		}

		/// Returns the NodeRef at position 0, or the given NodeRef if this is empty.
		pub fn head_or(&self, head_maybe: &NodeRef) -> NodeRef {
			if self.entries.len() == 0 {
				head_maybe.clone()
			} else {
				let (ref h, ref _x) = self.entries[0];
				h.clone()
			}
		}

		/// Finds the NodeStack pointing to the given key. True if the key was found exactly.
		/// If the key was not found, the NodeStack points to the first key greater than the given node.
		/// In this case the NodeStack may not point to a valid node and may need revalidation.
		pub fn construct(n: NodeRef, k: &[u8]) -> (NodeStack, bool) {
			// Perf note: because we always use the 'fattest node', the potential of polymorphic recursion
			// doesn't help us.
			let mut stack = NodeStack::empty();
			let found;
			let mut n = n.clone();

			loop {
				match n.apply(|node| node.find(k)) {
					Ok(idx) => {
						stack.push(n.clone(), idx);
						found = true;
						break;
					}
					Err(idx) => {
						stack.push(n.clone(), idx);

						if n.apply(MemNode::is_leaf) {
							// At this point, it's possible for idx to be >= n.bucket_count
							found = false;
							break;
						} else {
							// continue the loop
							n = n.apply(|node| node.child_ref(idx));
						}
					}
				}
			};

			(stack, found)
		}

		/// Advances this NodeStack.
		/// Precondition: the stack is not empty.
		pub fn advance(&mut self) -> Option<WeakBucketRef> {
			let is_leaf;

			{ // Borrow checker block
				// Asserts we are not empty.
				let mut cursorref = self.peek_mut().unwrap();
				is_leaf = cursorref.0.apply(MemNode::is_leaf);

				// This is why we have the borrow checker block. If we pop-and-push, LLVM optimizes it poorly
				if is_leaf {
					cursorref.1 += 1;
				}
			}

			if is_leaf {
				// If a leaf, increment by one, then revalidate.
				return self.ascend_maybe();
			} else {
				return self.descend();
			}
		}

		/// Left-descends down the nodestack until we reach the first left bucket on the next leaf node.
		/// Precondition: we are not at a leaf node, and we are not empty.
		fn descend(&mut self) -> Option<WeakBucketRef> {
			let mut nref;

			{ // borrow checker block
				// Asserts we are not empty.
				let cursorref = self.peek_mut().unwrap();
				debug_assert!(!cursorref.0.apply(MemNode::is_leaf));

				// If we just visited bucket n, we visit child n+1 and find that child's leftmost descendant.
				nref = cursorref.0.apply(|node| node.child_ref(cursorref.1 + 1));
				// The next time we revisit this node, look at bucket n+1.
				cursorref.1 += 1;
			};

			loop {
				debug_assert!(nref.apply(|node| node.child_count() > 0));

				if nref.apply(MemNode::is_leaf) {
					let r = Some(nref.apply(|node| node.bucket_ref(0)));
					self.push(nref, 0);
					return r
				}

				let nref2 = nref.apply(|node| node.child_ref(0));
				self.push(nref, 0);
				nref = nref2
			}
		}

		/// Ascend until we are pointing at a valid bucket. This is called after any operation
		/// may point the cursor past a valid leaf bucket.
		/// (So, if we are at a leaf node with 4 buckets, we need to ascend if the top index == 4.)
		/// Precondition: we are at a leaf node, and we are not empty.
		pub fn ascend_maybe(&mut self) -> Option<WeakBucketRef> {
			{ // borrow checker block
				// Asserts we are not empty
				let topcursor = self.peek_mut().unwrap();
				debug_assert!(topcursor.0.apply(MemNode::is_leaf));

				// We have no need to ascend
				if topcursor.1 < topcursor.0.apply(|node| node.bucket_count()) {
					return Some(topcursor.0.apply(|node| node.bucket_ref(topcursor.1)));
				}
			}

			// Otherwise, ascend
			loop {
				self.pop();

				if self.is_empty() {
					// End of the cursor!
					return None;
				}

				let topcursor = self.peek_mut().unwrap();

				if topcursor.1 < topcursor.0.apply(|node| node.bucket_count()) {
					// This is a branch bucket. The next call to advance() should left-descend to the next
					// leaf bucket (at 1 beyond the current index).
					return Some(topcursor.0.apply(|node| node.bucket_ref(topcursor.1)));
				}

				// Otherwise, we have exhausted the branch buckets of this node. Continue the loop.
			}
		}
	}
}

pub use self::nodestack::NodeStack;

mod btree_insert {
	use data::*;

	use tree::btree::NodeStack;
	use tree::bucketref::BucketRef;
	use tree::memnode::*;
	use tree::noderef::{HotHandle, FatNodeRef, NodeRef};

	/// Precondition: self is the head node.
	// TODO: can we make this iterative? the issue is hot handles' lifetime syntactically depends on the parent handle,
	// when it really depends on the lifetime of top.
	fn insert_helper_nosplit(top: &mut NodeRef, nhot: HotHandle, stack: &mut NodeStack) -> FatNodeRef {
		if let Some((parent, parent_idx)) = stack.pop() {
			let (mut parent_hot, was_copied) = parent.heat();
			parent_hot.apply_mut(|hn| hn.reassign_child(parent_idx, nhot));

			if was_copied {
				insert_helper_nosplit(top, parent_hot, stack)
			} else {
				// If not, we reached the termination condition, and the head node is not modified
				stack.head_or(&parent).upgrade()
			}
		} else {
			// Termination condition, and we recursed all the way back to the (hot) head node
			let mut r = top.upgrade();
			r.reassign(nhot);
			r
		}
	}

	// TODO: figure out how to have a simple return thingy
	fn insert_helper(top: &mut NodeRef, nhot: HotHandle, insert_result: InsertResult,
		stack: &mut NodeStack) -> FatNodeRef {
		if let Some((parent, parent_idx)) = stack.pop() {
			// Get the next node up the stack, loop while we have to modify nodes
			// TODO: weak references?
			let (mut parent_hot, was_copied) = parent.heat();
			parent_hot.apply_mut(|hn| hn.reassign_child(parent_idx, nhot));

			match insert_result {
				InsertResult::Ok => {
					if was_copied {
						// This hot parent was modified. Flushing will not be necessary,
						// but we have to continue looping until we no longer need to modify hot parents.
						insert_helper_nosplit(top, parent_hot, stack)
					} else {
						// Termination condition, and we have not modified the head node
						stack.head_or(&parent).upgrade()
					}
				}
				InsertResult::Flushed(split_bucket, newnode) => {
					// This might trigger another flush.
					let insert_result = parent_hot.apply_mut(
						|hn| hn.insert_at(parent_idx, split_bucket, Some(FatNodeRef::new_transient(newnode))));
					insert_helper(top, parent_hot, insert_result, stack)
				}
			}
		} else {
			// We have recursed all the way back to the head node.
			let mut r = top.upgrade();
			r.reassign(nhot);

			match insert_result {
				InsertResult::Ok => {
					// Termination condition, and we have recursed all the way back to the (hot) head node
					r
				}
				InsertResult::Flushed(split_bucket, newnode) => {
					// Need to create a new head node, and return it
					FatNodeRef::new_transient(
						MemNode::new_from_two(r, split_bucket, FatNodeRef::new_transient(newnode)))
				}
			}
		}
	}

	// TODO: flushed should probably return a FatNodeRef as its 2nd node return value.
	pub fn insert<D: Datum>(top: &mut NodeRef, k: &[u8], v: &D) -> FatNodeRef {
		// TODO use an array stack. Minimize allocs
		// Depth is 0-indexed
		let (mut stack, exists) = NodeStack::construct(top.clone(), k);
		if exists {
			panic!("not implemented")
		}

		// Prepare to insert
		let (node, idx) = stack.pop().unwrap();
		let (mut nhot, _) = node.heat();
		let insert_result = nhot.apply_mut(|hn| hn.insert_at(idx, BucketRef::new_transient(k, v), None));

		insert_helper(top, nhot, insert_result, &mut stack)
	}
}

mod btree_get {
	use data::*;

	use tree::counter::*;
	use tree::memnode::*;
	use tree::noderef::NodeRef;

	pub fn get(mut n: NodeRef, k: &[u8]) -> Option<ByteRc> {
		loop {
			match n.apply(|node| node.find(k)) {
				Ok(idx) => {
					return Some(n.apply(|node| node.bucket_ref(idx).value()))
				}
				Err(idx) => {
					if n.apply(MemNode::is_leaf) {
						return None
					} else {
						// Continue the loop
						n = n.apply(|node| node.child_ref(idx));
					}
				},
			}
		}
	}

	/// Searches the tree, ignoring any transactions equal or older in time than the given txid.
	pub fn get_recent(mut n: NodeRef, k: &[u8], trailing_txid: Counter) -> Option<ByteRc> {
		loop {
			if !n.apply_persistent(|pnode| trailing_txid.circle_lt(pnode.txid())) {
				return None
			}

			match n.apply(|node| node.find(k)) {
				Ok(idx) => {
					let tx_test = n.apply(|node| trailing_txid.circle_lt(node.bucket_ref(idx).txid()));

					return if tx_test {
						Some(n.apply(|node| node.bucket_ref(idx).value()))
					} else {
						None
					}
				}
				Err(idx) => {
					if n.apply(MemNode::is_leaf) {
						return None
					} else {
						// Continue the loop
						n = n.apply(|node| node.child_ref(idx));
					}
				},
			}
		}
	}
}

pub struct BTreeCursor {
	stack: NodeStack,
	current_bucket: Option<WeakBucketRef>,
}

impl BTreeCursor {
	fn construct(head: NodeRef, k: &[u8]) -> BTreeCursor {
		let (mut stack, _) = NodeStack::construct(head, k);
		let bucket = stack.ascend_maybe();

		BTreeCursor {
			stack: stack,
			current_bucket: bucket,
		}
	}

	fn empty() -> BTreeCursor {
		BTreeCursor {
			stack: NodeStack::empty(),
			current_bucket: None,
		}
	}
}

impl Cursor for BTreeCursor {
	// TODO: these should be data streams.
	/// The type of a data stream.
	type GetDatum = ByteRc;

	/// The type returned by the get method.
	type Get = ByteRc;

	fn key(&self) -> Option<ByteRc> {
		self.current_bucket.as_ref().map(WeakBucketRef::key)
	}

	fn value(&self) -> Option<Self::Get> {
		self.current_bucket.as_ref().map(WeakBucketRef::value)
	}

	fn advance(&mut self) -> bool {
		self.current_bucket = self.stack.advance();

		self.current_bucket.is_some()
	}
}

// A simple in-memory persistent b-tree.
// TODO: naming. Transient, persistent. Two separate structs maybe?
pub struct PersistentBTree {
	// TODO: this shouldn't be an option.
	head: Option<FatNodeRef>,
	/// Gets the max txid of this PersistentBTree (exclusive). The next transient material to be persisted
	/// will have this txid.
	// TODO: test this invariant.
	leading_txid: Counter,
}

impl PersistentBTree {
	pub fn new() -> PersistentBTree {
		PersistentBTree {
			head: None,
			leading_txid: Counter::new(0),
		}
	}

	/// Gets the max txid of this PersistentBTree (exclusive).
	fn txid(&self) -> Counter {
		self.leading_txid
	}

	/// Internal method for snapshot diffs.
	fn get_recent<K: Key + ?Sized>(&mut self, k: &K, trailing_txid: Counter) -> Option<ByteRc> {
		self.head.as_ref().and_then(|strongref| btree_get::get_recent(strongref.noderef(), k.bytes(), trailing_txid))
	}

	/// Makes a persistent clone of this PersistentBTree. Does *not* update the current txid, of course.
	fn shallow_clone(&mut self) -> Self {
		let cloned_head;

		match self.head.as_mut() {
			Some(strongref) => {
				strongref.immute(self.leading_txid);
				cloned_head = Some(strongref.shallow_clone());
			}
			None => {
				cloned_head = None;
			}
		}

		PersistentBTree {
			head: cloned_head,
			leading_txid: self.leading_txid,
		}
	}

	/// Like shallow clone, except not mutable. Panics if this tree is not persistent.
	fn persistent_clone(&self) -> Self {
		PersistentBTree {
			head: self.head.as_ref().map(|strongref| strongref.shallow_clone()),
			leading_txid: self.leading_txid,
		}
	}

	fn cursor(&self, k: &[u8]) -> BTreeCursor {
		match self.head.as_ref() {
			Some(strongref) => BTreeCursor::construct(strongref.noderef(), k),
			None => BTreeCursor::empty(),
		}
	}
}

impl ByteMap for PersistentBTree {
	type GetDatum = ByteRc;
	type Get = ByteRc;

	fn get<K: Key + ?Sized>(&mut self, k: &K) -> Option<Self::Get> {
		self.head.as_ref().and_then(|strongref| btree_get::get(strongref.noderef(), k.bytes()))
	}

	fn check_invariants(&self) {
		self.head.as_ref().map(|strongref| strongref.check_invariants());
	}
}

impl MutableByteMap for PersistentBTree {
	fn insert<K: Key + ?Sized, V: Datum>(&mut self, k: &K, v: &V) -> () {
		let newhead;

		match self.head.as_ref() {
			Some(strongref) => {
				newhead = btree_insert::insert(&mut strongref.noderef(), k.bytes(), v);
			}
			None => {
				newhead = FatNodeRef::new_transient(MemNode::new_from_one(BucketRef::new_transient(k.bytes(), v)))
			}
		}

		self.head = Some(newhead);
	}

	// fn delete<K: Key + ?Sized>(&mut self, k: &K) -> bool {
	// 	let mut dummy = NodePtr::empty();
	// 	mem::swap(&mut dummy, &mut self.head);
	// 	let (newhead, r) = dummy.delete_for_root(k.bytes());
	// 	self.head = newhead;
	// 	r
	// }

	fn delete<K: Key + ?Sized>(&mut self, k: &K) -> bool {
		panic!("not implemented")
	}
}

pub struct PersistentDiff {
	v: PersistentBTree,
	rear_counter: Counter,
}

impl ByteMap for PersistentDiff {
	type GetDatum = <PersistentBTree as ByteMap>::GetDatum;
	type Get = <PersistentBTree as ByteMap>::Get;

	fn get<K: Key + ?Sized>(&mut self, k: &K) -> Option<Self::Get> {
		self.v.get_recent(k, self.rear_counter)
	}

	fn check_invariants(&self) {
		// TODO: likely overkill
		self.v.check_invariants()
	}
}

impl NavigableByteMap for PersistentDiff {
	type Cursor = BTreeCursor;

	fn cursor(&self, k: &[u8]) -> Self::Cursor {
		panic!("not yet implemented")
	}
}

// PersistentBTrees can serve as snapshots, as long as we don't try to write to a persistent node.
pub struct PersistentSnap {
	v: PersistentBTree,
}

impl ByteMap for PersistentSnap {
	type GetDatum = <PersistentBTree as ByteMap>::GetDatum;
	type Get = <PersistentBTree as ByteMap>::Get;

	fn get<K: Key + ?Sized>(&mut self, k: &K) -> Option<Self::Get> {
		self.v.get(k)
	}

	fn check_invariants(&self) {
		self.v.check_invariants()
	}
}

impl ByteMapSnapshot for PersistentSnap {
	type Diff = PersistentDiff;

	fn txid(&self) -> Counter {
		self.v.txid()
	}

	fn diff(&self, past: Counter) -> Self::Diff {
		// TODO assertions about the counter
		PersistentDiff {
			v: self.v.persistent_clone(),
			rear_counter: past,
		}
	}
}

impl NavigableByteMap for PersistentSnap {
	type Cursor = BTreeCursor;

	fn cursor(&self, k: &[u8]) -> Self::Cursor {
		self.v.cursor(k)
	}
}

impl FunctionalByteMap for PersistentBTree {
	// I N C E P T I O N
	type Snap = PersistentSnap;

	fn snap(&mut self) -> Self::Snap {
		let clone = self.shallow_clone();
		// We might bump the leading txid even if the transaction does nothing. This is by design.
		self.leading_txid = self.leading_txid.inc();

		PersistentSnap {
			v: clone,
		}
	}
}
