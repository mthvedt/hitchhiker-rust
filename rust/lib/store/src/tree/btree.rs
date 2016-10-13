use std::borrow::Borrow;
use std::cell::{RefCell};
use std::mem;
use std::ops::Deref;
use std::rc::Weak;

use data::*;

use tree::bucket::*;
use tree::counter::*;
use tree::hotnode::*;
use tree::nodeptr::*;
use tree::traits::*;

const MAX_DEPTH: u8 = 32;

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

pub trait CowByteMap: MutableByteMap {
	type Snap: ByteMap;

	fn snap(&mut self) -> Self::Snap;
}

type NodeCursor = (NodeHandle, u16);

struct NodeStack {
	entries: Vec<NodeCursor>,
}

impl NodeStack {
	pub fn new() -> NodeStack {
		NodeStack {
			// master_node: topnode,
			entries: Vec::with_capacity(MAX_DEPTH as usize),
		}
	}

	fn push(&mut self, node: NodeHandle, child_index: u16) {
		debug_assert!(self.entries.len() < MAX_DEPTH as usize);
		self.entries.push((node, child_index));
	}

	fn pop(&mut self) -> Option<NodeCursor> {
		self.entries.pop()
	}

	/// Returns the NodeHandle at position 0, or the given NodeHandle if this is empty.
	fn head_or(&self, head_maybe: &NodeHandle) -> NodeHandle {
		if self.entries.len() == 0 {
			head_maybe.clone()
		} else {
			let (ref h, ref _x) = self.entries[0];
			h.clone()
		}
	}
}

/// A handle to a ready node which can be quickly dereferenced. The existence of this handle
/// may pin resources.
/// TODO: delete this class, use pointers to FatNodes. Rc in debug, raw in release.
#[derive(Clone)]
pub enum NodeHandle {
 	// TODO: something faster. Ideal situation is a head snapshot/job pointer and a raw pointer.
	Hot(Weak<RefCell<HotNode>>),
	// TODO: shouldn't expose PersistentNode inner like this.
	Warm(Weak<(Counter, HotNode)>),
}

impl NodeHandle {
	fn to_fat_node(&self) -> FatNode {
		match self {
			&NodeHandle::Hot(ref w_) => FatNode::Transient(w_.upgrade().unwrap()),
			&NodeHandle::Warm(ref w_) => FatNode::Persistent(PersistentNode{ v: w_.upgrade().unwrap() }),
		}
	}

	pub fn apply<F, R> (&self, f: F) -> R where F: for<'x> FnOnce(&'x HotNode) -> R,
	{
		match self {
			&NodeHandle::Hot(ref w_rfc_hn) => {
				// Stupid borrow checker tricks
				let x = w_rfc_hn.upgrade().unwrap();
				let r = f(x.deref().borrow().deref());
				r
			}
			&NodeHandle::Warm(ref w_hn) => f(&w_hn.upgrade().unwrap().deref().deref().1),
		}
	}

	fn child_handle(&self, idx: u16) -> NodeHandle {
		self.apply(|hn| hn.child(idx).handle())
	}

	/* CRUD */

	// This was made into a tail recursive function as a result of a historical fight with the borrow checker.
	// TODO: can we make this iterative?
	// TODO: actually this should not be a property of an internal class.
	fn find_node_chain_helper(&self, k: &[u8], stack: &mut NodeStack) -> bool {
		match self.apply(|n| n.find(k)) {
			Ok(idx) => {
				stack.push(self.clone(), idx);
				true
			}
			Err(idx) => {
				stack.push(self.clone(), idx);

				if self.apply(HotNode::is_leaf) {
					false
				} else {
					let child = self.child_handle(idx);
					child.find_node_chain_helper(k, stack)
				}
			}
		}
	}

	fn find_node_chain(&self, k: &[u8]) -> (NodeStack, bool) {
		let mut stack = NodeStack::new();
		let r = self.find_node_chain_helper(k, &mut stack);
		(stack, r)
	}

	/// Precondition: self is the head node.
	fn insert_helper_nosplit(&mut self, nhot: HotHandle, stack: &mut NodeStack) -> FatNode {
		if let Some((parent, parent_idx)) = stack.pop() {
			let parent_handle = parent.to_fat_node();
			let (mut parent_hot, was_copied) = parent_handle.heat();
			parent_hot.apply_mut(|hn| hn.reassign_child(parent_idx, nhot));

			if was_copied {
				self.insert_helper_nosplit(parent_hot, stack)
			} else {
				// If not, we reached the termination condition, and the head node is not modified
				stack.head_or(&parent).to_fat_node()
			}
		} else {
			// Termination condition, and we recursed all the way back to the (hot) head node
			let mut r = self.to_fat_node();
			r.reassign(nhot);
			r
		}
	}

	// TODO: figure out how to have a simple return thingy
	fn insert_helper(&mut self, nhot: HotHandle, insert_result: InsertResult, stack: &mut NodeStack) -> FatNode {
		if let Some((parent, parent_idx)) = stack.pop() {
			// Get the next node up the stack, loop while we have to modify nodes
			// TODO: weak references?
			let parent_handle = parent.to_fat_node();
			// TODO: HotNodes shouldn't work this way. This is lasagna logic
			let (mut parent_hot, was_copied) = parent_handle.heat();
			parent_hot.apply_mut(|hn| hn.reassign_child(parent_idx, nhot));

			match insert_result {
				InsertResult::Ok => {
					if was_copied {
						// This hot parent was modified. Flushing will not be necessary,
						// but we have to continue looping until we no longer need to modify hot parents.
						self.insert_helper_nosplit(parent_hot, stack)
					} else {
						// Termination condition, and we have not modified the head node
						stack.head_or(&parent).to_fat_node()
					}
				}
				InsertResult::Flushed(split_bucket, newnode) => {
					// This might trigger another flush.
					let insert_result = parent_hot.apply_mut(
						|hn| hn.insert_at(parent_idx, split_bucket, Some(NodePtr::new_transient(newnode))));
					self.insert_helper(parent_hot, insert_result, stack)
				}
			}
		} else {
			// We have recursed all the way back to the head node.
			let mut r = self.to_fat_node();
			r.reassign(nhot);

			match insert_result {
				InsertResult::Ok => {
					// Termination condition, and we have recursed all the way back to the (hot) head node
					r
				}
				InsertResult::Flushed(split_bucket, newnode) => {
					// Need to create a new head node, and return it
					FatNode::new_transient(
						HotNode::new_from_two(NodePtr::wrap(r), split_bucket, NodePtr::new_transient(newnode)))
				}
			}
		}
	}

	// TODO: flushed should probably return a FatNode as its 2nd node return value.
	pub fn insert<D: Datum>(&mut self, k: &[u8], v: &D) -> FatNode {
		// TODO use an array stack. Minimize allocs
		// Depth is 0-indexed
		let (mut stack, exists) = self.find_node_chain(k);
		if exists {
			panic!("not implemented")
		}

		// Prepare to insert
		let (node, idx) = stack.pop().unwrap();
		let _ntmp = node.to_fat_node();
		let (mut nhot, _) = _ntmp.heat();
		let insert_result = nhot.apply_mut(|hn| hn.insert_at(idx, Bucket::new(k, v), None));

		self.insert_helper(nhot, insert_result, &mut stack)
	}

	// TODO: can we make this iterative?
	pub fn get(&self, k: &[u8]) -> Option<ByteRc> {
		match self.apply(|n| n.find(k)) {
			Ok(idx) => Some(self.apply(|n| n.value(idx))),
			Err(idx) => {
				if self.apply(HotNode::is_leaf) {
					None
				} else {
					let child = self.child_handle(idx);
					child.get(k)
				}
			},
		}
	}
}

// // The idea behind having Jobs for everything is we eventually want to put such into Futures.

// /// A job for getting a key.
// struct GetJob {
// }

// impl GetJob {
// 	// TODO: can we make this iterative?
// 	pub fn get<H>(&self, k: &[u8], h: Handle) -> Option<ByteRc> {
// 		match self.apply(|n| n.find(k)) {
// 			Ok(idx) => Some(self.apply(|n| n.value(idx))),
// 			Err(idx) => {
// 				if self.apply(HotNode::is_leaf) {
// 					None
// 				} else {
// 					let child = self.child_handle(idx);
// 					child.get(k)
// 				}
// 			},
// 		}
// 	}
// }

// A simple in-memory persistent b-tree.
pub struct PersistentBTree {
	head: NodePtr,
	// TODO: safety check and tests?
	// The txid of the next transaction to be committed. Should always be one more than the previous one.
	// TODO: test this invariant.
	leading_txid: Counter,
}

impl PersistentBTree {
	pub fn new() -> PersistentBTree {
		PersistentBTree {
			head: NodePtr::empty(),
			leading_txid: Counter::new(0),
		}
	}
}

impl ByteMap for PersistentBTree {
	type GetDatum = ByteRc;
	type Get = ByteRc;

	fn get<K: Key + ?Sized>(&mut self, k: &K) -> Option<Self::Get> {
		self.head.get(k.bytes())
	}

	fn check_invariants(&self) {
		self.head.check_invariants();
	}
}

impl MutableByteMap for PersistentBTree {
	fn insert<K: Key + ?Sized, V: Datum>(&mut self, k: &K, v: &V) -> () {
		// Dummy to make the compiler behave. Since we're dealing in Options and Boxes, shouldn't have a runtime cost.
		let mut dummy = NodePtr::empty();
		mem::swap(&mut dummy, &mut self.head);
		self.head = dummy.insert(k.bytes(), v);
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

impl CowByteMap for PersistentBTree {
	// I N C E P T I O N
	type Snap = PersistentSnap;

	fn snap(&mut self) -> Self::Snap {
		self.head.cool(self.leading_txid);
		// We might bump the leading txid even if the transaction does nothing. This is by design.
		self.leading_txid.inc();

		PersistentSnap {
			v: PersistentBTree {
				head: self.head.shallow_clone(),
				leading_txid: self.leading_txid,
			}
		}
	}
}
