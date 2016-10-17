use std::borrow::Borrow;
use std::cell::{RefCell};
use std::mem;
use std::ops::Deref;
use std::rc::Weak;

use data::*;

use tree::bucket::*;
use tree::counter::*;
use tree::memnode::*;
use tree::noderef::*;

// TODO: move to module level doc the below.
// A key is anything that can be (quickly, efficiently) converted to raw bytes.
// A value is a Datum, a set of bytes that can be streamed.

pub trait ByteMap {
	type GetDatum: Datum;
	type Get: Borrow<Self::GetDatum>;

	/// This is mutable because gets may introduce read conflicts, and hence mutate the underlying datastructure.
	// TODO this does not need to be mutable.
	fn get<K: Key + ?Sized>(&mut self, k: &K) -> Option<Self::Get>;

	/// Debug method to check this data structures's invariants.
	// TODO: isolate.
	fn check_invariants(&self);
}

pub trait MutableByteMap: ByteMap {
	fn insert<K: Key + ?Sized, V: Datum>(&mut self, k: &K, v: &V) -> ();

	fn delete<K: Key + ?Sized>(&mut self, k: &K) -> bool;
}

pub trait ByteMapSnapshot: ByteMap {
	type Diff: ByteMap;

	fn txid(&self) -> Counter;

	fn diff(&self, past: Counter) -> Self::Diff;
}

pub trait FunctionalByteMap: MutableByteMap {
	type Snap: ByteMapSnapshot;

	fn snap(&mut self) -> Self::Snap;
}

mod nodestack {
	use tree::noderef::NodeRef;
	use tree::memnode::*;

	const MAX_DEPTH: u8 = 32;

	pub type NodeCursor = (NodeRef, u16);

	pub struct NodeStack {
		entries: Vec<NodeCursor>,
	}

	impl NodeStack {
		pub fn new() -> NodeStack {
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

		/// Returns the NodeRef at position 0, or the given NodeRef if this is empty.
		pub fn head_or(&self, head_maybe: &NodeRef) -> NodeRef {
			if self.entries.len() == 0 {
				head_maybe.clone()
			} else {
				let (ref h, ref _x) = self.entries[0];
				h.clone()
			}
		}
	}

	pub fn construct(n: NodeRef, k: &[u8]) -> (NodeStack, bool) {
		// Perf note: because we always use the 'fattest node', the potential of polymorphic recursion
		// doesn't help us.
		let mut stack = NodeStack::new();
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
}

mod btree_insert {
	use data::*;

	use tree::btree::nodestack;
	use tree::bucket::Bucket;
	use tree::memnode::*;
	use tree::noderef::{HotHandle, FatNodeRef, NodeRef};

	/// Precondition: self is the head node.
	// TODO: can we make this iterative? the issue is hot handles' lifetime syntactically depends on the parent handle,
	// when it really depends on the lifetime of top.
	fn insert_helper_nosplit(top: &mut NodeRef, nhot: HotHandle, stack: &mut nodestack::NodeStack) -> FatNodeRef {
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
		stack: &mut nodestack::NodeStack) -> FatNodeRef {
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
		let (mut stack, exists) = nodestack::construct(top.clone(), k);
		if exists {
			panic!("not implemented")
		}

		// Prepare to insert
		let (node, idx) = stack.pop().unwrap();
		let (mut nhot, _) = node.heat();
		let insert_result = nhot.apply_mut(|hn| hn.insert_at(idx, Bucket::new(k, v), None));

		insert_helper(top, nhot, insert_result, &mut stack)
	}
}

mod btree_get {
	use data::*;

	use tree::counter::*;
	use tree::memnode::*;
	use tree::noderef::{HotHandle, NodeRef};

	fn get_helper<F>(mut n: NodeRef, k: &[u8], filter: F) -> Option<ByteRc>
	where F: Fn(&NodeRef) -> bool {
		loop {
			match n.apply(|node| node.find(k)) {
				Ok(idx) => {
					return Some(n.apply(|node| node.value(idx).clone()))
				}
				Err(idx) => {
					if n.apply(MemNode::is_leaf) {
						return None
					} else {
						n = n.apply(|node| node.child_ref(idx));
					}
				},
			}
		}
	}

	pub fn get(n: NodeRef, k: &[u8]) -> Option<ByteRc> {
		get_helper(n, k, |_| true)
	}

	pub fn get_recent(mut n: NodeRef, k: &[u8], trailing_txid: Counter) -> Option<ByteRc> {
		get_helper(n, k, |_| true)
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
				strongref.cool(self.leading_txid);
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
				newhead = FatNodeRef::new_transient(MemNode::new_from_one(Bucket::new(k.bytes(), v)))
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

impl FunctionalByteMap for PersistentBTree {
	// I N C E P T I O N
	type Snap = PersistentSnap;

	fn snap(&mut self) -> Self::Snap {
		let clone = self.shallow_clone();
		// We might bump the leading txid even if the transaction does nothing. This is by design.
		self.leading_txid.inc();

		PersistentSnap {
			v: clone,
		}
	}
}
