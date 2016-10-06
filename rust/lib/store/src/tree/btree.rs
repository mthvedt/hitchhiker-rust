use std::borrow::Borrow;
use std::cell::*;
use std::cmp::{Ord, Ordering};
use std::marker::PhantomData;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::ptr;
use std::rc::{Rc, Weak};

use data::*;
use data::slice::{ByteBox, ByteRc};

use tree::nodeptr::*;
use tree::util::*;

const NODE_CAPACITY: u16 = 16;

const MAX_DEPTH: u8 = 32;

// TODO: move to module level doc the below.
/// A key is anything that can be (quickly, efficiently) converted to raw bytes.
/// A value is a Datum, a set of bytes that can be streamed.
pub trait ByteMap {
	type D: Datum;

	/// We only accept references that can be quickly converted to keys and values,
	/// for performance reasons.
	fn insert<K: Key + ?Sized, V: Datum>(&mut self, k: &K, v: &V) -> ();

	/// This is mutable because gets may introduce read conflicts, and hence mutate the underlying datastructure.
	fn get<K: Key + ?Sized>(&mut self, k: &K) -> Option<&Self::D>;

	fn delete<K: Key + ?Sized>(&mut self, k: &K) -> bool;

	fn check_invariants(&self);
}

// pub trait ByteTree: ByteMap {

// }

#[derive(Clone)]
pub struct Bucket {
	k: ByteRc,
	v: ByteRc,
}

impl Bucket {
	fn new<V: Datum>(k: &[u8], v: &V) -> Bucket {
		Bucket {
			k: ByteRc::from_key(k),
			v: ByteRc::from_value(v),
		}
	}
}

#[derive(Clone)]
pub struct BucketPtr {
	v: Option<Bucket>,
}

impl BucketPtr {
	fn empty() -> BucketPtr {
		BucketPtr {
			v: None,
		}
	}

	fn new<V: Datum>(k: &[u8], v: &V) -> BucketPtr {
		BucketPtr {
			v: Some(Bucket::new(k, v)),
		}
	}

	fn wrap(b: Bucket) -> BucketPtr {
		BucketPtr { v: Some(b), }
	}

	fn unwrap(self) -> Bucket {
		self.v.unwrap()
	}

	pub fn key(&self) -> &[u8] {
		self.v.as_ref().unwrap().k.borrow()
	}

	pub fn warm_value(&self) -> &[u8] {
		self.v.as_ref().unwrap().v.borrow()
	}

	pub fn is_empty(&self) -> bool {
		self.v.is_none()
	}
}

// trait RefCell<T> {

// }

// struct DebugRefCell<T> {

// }

// impl RefCell<T> for DebugRefCell<T> {

// }

// struct FastRefCell<T> {

// }

// impl RefCell<T> for FastRefCell<T> {

// }

// trait TreeTypeclass {
// 	type RefCellT: RefCell,
// }

/*
TODO: how to insert? :( :( :(

We want to travel down the tree, converting any ColdRefs to WarmRefs.

These need to be compiled into a RefSet, which validates/invalidates refs.

A node pointer. Hm...

Async reads have these components:
* A master validating/invalidating pointer.
* The nodes themselves. They can be "warm", representing read-only disk data, or "hot", representing unflushed data.
* ***RAW POINTERS*** to the nodes. References don't work because of borrow checker nightmares.

Handles shouldn't be mutable. In general, prefer nonmutability--push mutability out to the frontier.
Warming a handle may return a new handle to a hot node or an existing one.
Whatever the case, it's now ready for borrowing.

Warm nodes are never mutable.

Hot nodes are mutable.

---

SO. Final decision.

We need to have the following:
* Reader handles. These present a way to look up a node. They can look up
  warm nodes or hot nodes. They are always immutable--recall that in Rust, mutability has syntactic meaning.
* Hot references. In addition to functioning like a handle that yields a hot node, they can be back-written
  to hot node parents.
	- In the case where the parent and child are hot, we must be able to borrow and mutate the child
	  without knowing the state of the parent or having a mutable reference. Thus, an internal hot reference
	  needs to live in some kind of cell, which we can borrow mutably without modifying the hot parent.
	  When the "modified" hot child is written back to the parent, its reference inside the parent is overwritten.
* Internal hot references (see above).
* Nodes themselves.

*/

// TODO: Cold Warm Hot -> Cold Ready Hot

// /// An address to a Node, which may be hot, ready or cold.
// /// TODO: address validators. Need to always be owned by a node address.
// struct NodeAddress {
// 	// TODO: something faster. Ideal situation is a head snapshot pointer and a raw pointer.
// 	n: *const NodeRef,
// }

// impl NodeAddress {
// 	fn new(n: &NodeRef) -> NodeAddress {
// 		NodeAddress { n: n as *const _ }
// 	}

// 	/// Readies the contents of this node handle, possibly incurring I/O and resource aquisition.
// 	/// Returns a NodeHandle. If resources were acquired, the returned NodeHandle owns that acquisition.
// 	// TODO make this a future.
// 	fn lookup(&self, v: HandleRepo) -> NodeHandle {
// 		NodeHandle {
// 			n: self.n,
// 		}
// 	}
// }

/// A handle to a ready node which can be quickly dereferenced. The existence of this handle
/// may pin resources.
#[derive(Clone)]
pub enum NodeHandle {
 	// TODO: something faster. Ideal situation is a head snapshot/job pointer and a raw pointer.
	// n: *const NodeRef,
	// job: JobPtr,

	Hot(Weak<RefCell<HotNode>>),
	Warm(Weak<HotNode>),
}

impl NodeHandle {
	// TODO:

	/// The design principle is to avoid trait polymorphism. We want at most one unoptimizable
	/// enum switch when we look up the dynamic type of a node or handle. Furthermore,
	/// we want the inliner to inline the results of such calls. This design pattern serves a dual purpose;
	/// one, once the type of an enum is known, the optimizer can often inline and infer enum values
	/// for related functions; two, each application creates a function call context that is "de-enumized",
	/// so that un-inlinable scenarios still do not result in us re-testing the same enum over and over.
	///
	/// "But polymorphism isn't that slow." Yes it is. Languages which lean heavily on it, such as Java,
	/// have complex optimizers that can inline and optimize large chains of virtual calls. Languages
	/// which use statically compiled virtual "fat pointers", like Rust or C++, cannot do that.
	// pub fn apply<F, N, R>(&self, f: F) -> R where
	// F = FnOnce<N, Output = R>,
	// N = Node,
	// {}

	fn upgrade(&self) -> NodeRef {
		match self {
			// Same call, different objects. Necessary because of the monomorphism restriction.
			&NodeHandle::Hot(ref w_) => NodeRef::Hot(w_.upgrade().unwrap()),
			&NodeHandle::Warm(ref w_) => NodeRef::Warm(w_.upgrade().unwrap()),
		}
	}

	pub fn apply<F, R> (&self, f: F) -> R where
	F: for<'x> FnOnce(&'x HotNode) -> R,
	{
		match self {
			// Same call, different objects. Necessary because of the monomorphism restriction.
			&NodeHandle::Hot(ref w_rfc_hn) => {
				// Stupid borrow checker tricks
				let x = w_rfc_hn.upgrade().unwrap();
				let r = f(x.deref().borrow().deref());
				r
			}
			&NodeHandle::Warm(ref w_hn) => f(w_hn.upgrade().unwrap().deref()),
		}
	}

	// fn get_ref(&self) -> &NodeRef {
	// 	// self.n.
	// }

	// fn get_raw<'a>(&'a self) -> HotRef<'a> {
	// 	self.upgrade().acquire()
	// }

	// fn heat(&self) -> (HotHandle, bool) {
	// 	self.upgrade().heat()
	// }

	fn child_handle(&self, idx: u16) -> NodeHandle {
		self.apply(|hn| hn.child_ref(idx).handle())
	}

	/* CRUD */
	// This was made into a tail recursive function as a result of a historical fight with the borrow checker.
	// It's more elegant written recursively, anyway... finding a child should be a property of the current noderef.
	// TODO: actually this should not be a property of an internal class.
	// TODO: we shouldn't pass self by copy
	fn find_node_chain_helper(&self, k: &[u8], stack: &mut NodeStack) -> bool {
		// let search = unsafe { (&*(stack_top)).warm().find_bucket(k) };
		let search = self.apply(|n| n.find_bucket(k));

		match search {
			Ok(idx) => {
				stack.push(self.clone(), idx);
				true
			}
			Err(idx) => {
				let is_leaf = self.apply(HotNode::is_leaf);
				stack.push(self.clone(), idx);

				if is_leaf {
					false
				} else {
					let child = self.child_handle(idx);
					child.find_node_chain_helper(k, stack)
				}
			}
		}
	}

	fn find_node_chain(&mut self, k: &[u8]) -> (NodeStack, bool) {
		let mut stack = NodeStack::new();
		let r = self.find_node_chain_helper(k, &mut stack);
		(stack, r)
		// // The borrow checker fails if we let
		// let mut stack_top = self as *mut Self;
		// // debug_assert!() TODO compare pointers

		// loop {
		// 	let search = unsafe { (&*(stack_top)).warm().find_bucket(k) };

		// 	match search {
		// 		Ok(idx) => {
		// 			cursor.push(stack_top, idx);
		// 			return (cursor, true)
		// 		}
		// 		Err(idx) => {
		// 			let mut new_stack_top = stack_top.warm().child_ptr_mut(idx).unwrap_mut();
		// 			cursor.push(stack_top, idx);

		// 			if stack_top.warm().is_leaf() {
		// 				return (cursor, false)
		// 			}

		// 			stack_top = new_stack_top
		// 		}
		// 	}
		// }
	}

	fn insert_helper_noflush(nhot: HotHandle, stack: &mut NodeStack) -> Option<NodeRef> {
		if let Some((parent, parent_idx)) = stack.pop() {
			let _ntmp = parent.upgrade();
			let (mut parent_hot, was_copied) = _ntmp.heat();
			parent_hot.apply_mut(|hn| hn.reassign_child(parent_idx, nhot));

			if !was_copied {
				// Termination condition
				None
			} else {
				Self::insert_helper_noflush(parent_hot, stack)
			}
		} else {
			// Termination condition
			None
		}
	}

	fn insert_helper(nhot: HotHandle, insert_result: InsertResult, stack: &mut NodeStack) -> Option<NodeRef> {
		if let Some((parent, parent_idx)) = stack.pop() {
			// Get the next node up the stack, loop while we have to modify nodes
			// TODO: weak references?
			let _ntmp = parent.upgrade();
			// TODO: HotNodes shouldn't work this way.
			let (mut parent_hot, was_copied) = _ntmp.heat();

			match insert_result {
				InsertResult::Ok => {
					if was_copied {
						// This hot parent was modified. Flushing will not be necessary,
						// but we have to continue looping until we no longer need to modify hot parents.
						parent_hot.apply_mut(|hn| hn.reassign_child(parent_idx, nhot));
						Self::insert_helper_noflush(parent_hot, stack)
					} else {
						None
					}
				}
				InsertResult::Flushed(split_bucket, newnode) => {
					// This might trigger another flush.
					let insert_result = parent_hot.apply_mut(
						|hn| hn.insert_at(parent_idx, split_bucket, Some(NodePtr::new(newnode))));
					Self::insert_helper(parent_hot, insert_result, stack)
				}
			}
		} else {
			// Made it, ma! Top of the world! (We have looped all the way back to the head node.)
			match insert_result {
				InsertResult::Ok => {
					// self.get_ref().reassign(nhot);
					panic!("how do i do this?")
				}
				InsertResult::Flushed(split_bucket, newnode) => {
					// Need to create a new head node, and return it.
					// self.get_ref().reassign(nhot);
					panic!("how do i do this?")
					// return Some(NodeRef::new(HotNode::new_from_two(NodePtr::wrap(self),
					// 	split_bucket, NodePtr::new(newnode))))
				}
			}
		}
	}

	// TODO: flushed should probably return a NodeRef as its 2nd node return value.
	pub fn insert<D: Datum>(&mut self, k: &[u8], v: &D) -> Option<NodeRef> {
		// TODO use an array stack. Minimize allocs
		// Depth is 0-indexed
		let (mut stack, exists) = self.find_node_chain(k);
		if exists {
			panic!("not implemented")
		}

		let (mut node, idx) = stack.pop().unwrap();
		// First, insert for the leaf.
		let _ntmp = node.upgrade();
		let (mut nhot, _) = _ntmp.heat();
		let mut insert_result = nhot.apply_mut(|hn| hn.insert_at(idx, Bucket::new(k, v), None));

		Self::insert_helper(nhot, insert_result, &mut stack)
	}

	// pub fn insert<D: Datum>(&mut self, k: &[u8], v: &D) -> Option<NodeRef> {
	// 	// TODO use an array stack. Minimize allocs
	// 	// Depth is 0-indexed
	// 	let (mut stack, exists) = self.find_node_chain(k);
	// 	if exists {
	// 		panic!("not implemented")
	// 	}

	// 	let (mut node, idx) = stack.pop().unwrap();
	// 	// First, insert for the leaf.
	// 	let _ntmp = node.upgrade();
	// 	let (mut nhot, _) = _ntmp.heat();
	// 	let mut insert_result = nhot.apply_mut(|hn| hn.insert_at(idx, Bucket::new(k, v), None));

	// 	loop {
	// 		if let Some((parent, parent_idx)) = stack.pop() {
	// 			// Get the next node up the stack, loop while we have to modify nodes
	// 			// TODO: weak references?
	// 			let _ntmp = parent.upgrade();
	// 			let (mut parent_hot, was_copied) = _ntmp.heat();

	// 			match insert_result {
	// 				InsertResult::Ok => {
	// 					if was_copied {
	// 						parent_hot.apply_mut(|hn| hn.reassign_child(parent_idx, nhot));
	// 						nhot = parent_hot;

	// 						loop {
	// 							// This pointer was modified. Flushing will not be necessary,
	// 							// but we have to continue looping until we no longer need to modify parent pointers.
	// 							if let Some((parent, parent_idx)) = stack.pop() {
	// 								let _ntmp = parent.upgrade();
	// 								let (mut parent_hot, was_copied) = _ntmp.heat();
	// 								parent_hot.apply_mut(|hn| hn.reassign_child(parent_idx, nhot));

	// 								// Termination and reloop
	// 								if !was_copied {
	// 									return None
	// 								}
	// 								nhot = parent_hot;
	// 							} else {
	// 								// Termination condition
	// 								return None
	// 							}
	// 						}
	// 					} else {
	// 						return None
	// 					}
	// 				}
	// 				InsertResult::Flushed(split_bucket, newnode) => {
	// 					// This might trigger a flush, so we must continue in the flush loop.
	// 					insert_result = parent_hot.apply_mut(
	// 						|hn| hn.insert_at(parent_idx, split_bucket, Some(NodePtr::new(newnode))));
	// 				}
	// 			}

	// 			nhot = parent_hot;
	// 		} else {
	// 			// Made it, ma! Top of the world! (We have looped all the way back to the head node.)
	// 			match insert_result {
	// 				InsertResult::Ok => {
	// 					// self.get_ref().reassign(nhot);
	// 					panic!("how do i do this?")
	// 				}
	// 				InsertResult::Flushed(split_bucket, newnode) => {
	// 					// Need to create a new head node, and return it.
	// 					// self.get_ref().reassign(nhot);
	// 					panic!("how do i do this?")
	// 					// return Some(NodeRef::new(HotNode::new_from_two(NodePtr::wrap(self),
	// 					// 	split_bucket, NodePtr::new(newnode))))
	// 				}
	// 			}
	// 		}
	// 	}
	// }
}

// TODO rearrange this file
/// A handle to a hot node which can be quickly dereferenced. The existence of this handle
/// may pin resources, including an owned HotNode.
#[cfg(debug_assertions)]
pub enum HotHandle<'a> {
	Existing(RefMut<'a, HotNode>),
	// The NodeRef is used for debug assertions. It is forbidden to reassign a NodeRef to a HotHandle
	// that did not 'come from' that NodeRef.
	// We use a Rc<RefCell> here so it's easier to pass into NodeRef.
	New(Option<&'a NodeRef>, Rc<RefCell<HotNode>>),
}

#[cfg(not(debug_assertions))]
pub enum HotHandle<'a> {
	Existing(RefMut<HotNode>),
	// Use a zero-sized type so that HotRef has the same syntactic layout in debug and release mode.
	New((), Rc<RefCell<HotNode>>),
}

impl<'a> HotHandle<'a> {
	// fn as_ref(&self) -> &HotNode {
	// 	match self {
	// 		&HotRef::Existing(ref refmut) => refmut.deref(),
	// 		&HotRef::New(_, ref box_cell) => box_cell.as_ref().borrow().deref(),
	// 	}
	// }

	// fn as_mut(&mut self) -> &mut HotNode {
	// 	match self {
	// 		&mut HotHandle::Existing(ref mut refmut) => refmut.deref_mut(),
	// 		// Note we get_mut the RC cell, to enforce the constraint that this HotNode is not borrowed elsewhere.
	// 		&mut HotHandle::New(_, ref mut rc_cell) => Rc::get_mut(rc_cell).unwrap().deref_mut().borrow_mut(),
	// 	}
	// }

	#[cfg(debug_assertions)]
	pub fn apply_mut<F, R> (&mut self, f: F) -> R where
	F: FnOnce(&mut HotNode) -> R
	{
		match self {
			// Same call, different objects. Necessary because of the monomorphism restriction.
			&mut HotHandle::Existing(ref mut rfm_hn) => f(rfm_hn.deref_mut()),
			&mut HotHandle::New(_, ref mut rc_rfc_hn) => f(Rc::get_mut(rc_rfc_hn).unwrap().borrow_mut().deref_mut()),
		}
	}

	fn existing(n: RefMut<HotNode>) -> HotHandle {
		HotHandle::Existing(n)
	}

	#[cfg(debug_assertions)]
	fn heated(r: &'a NodeRef, n: HotNode) -> HotHandle<'a> {
		HotHandle::New(Some(r), Rc::new(RefCell::new(n)))
	}

	#[cfg(not(debug_assertions))]
	fn heated(r: &'a NodeRef, n: HotNode) -> HotHandle<'a> {
		HotHandle::New((), Rc::new(RefCell::new(n)))
	}

	// #[cfg(debug_assertions)]
	// fn new(n: HotNode) -> HotRef<'static> {
	// 	HotRef::New(None, Box::new(RefCell::new(n)))
	// }

	// #[cfg(not(debug_assertions))]
	// fn new(n: HotNode) -> HotRef<'static> {
	// 	HotRef::New((), Box::new(RefCell::new(n)))
	// }

	// fn empty() -> HotRef<'static> {
	// 	HotRef::new(HotNode::empty())
	// }
}

// // Unfortunately, there is no way around using weak pointers for the node stack.
// #[derive(Clone)]
// pub struct NodeRefWeak {
// 	noderef: *mut NodeRef,
// }

// impl NodeRefWeak {
// 	fn new(n: &mut NodeRef) -> NodeRefWeak {
// 		NodeRefWeak {
// 			noderef: n as *mut _,
// 		}
// 	}

// 	fn as_ref(&self) -> &NodeRef {
// 		unsafe { &*self.noderef }
// 	}

// 	fn as_mut(&self) -> &mut NodeRef {
// 		unsafe { &mut *self.noderef }
// 	}
// }

// TODO private
type NodeCursor = (NodeHandle, u16);

struct NodeStack {
	// master_node: NodeRef,
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

	// fn peek_unsafe(&'a self) -> &'a mut NodeRef {
	// 	let (r, _) = self.entries[self.entries.len() - 1];
	// 	r
	// }

	fn pop(&mut self) -> Option<NodeCursor> {
		self.entries.pop()
	}
}

/// A lifetimed, temporary reference to a hot node.
/// This reference may contain a new hot node, or an already existing one.
// TODO: do we need both this and HotHandle?
pub enum HotRef<'a> {
	Hot(Ref<'a, HotNode>),
	Warm(&'a HotNode),
}

impl<'a> HotRef<'a> {
	fn lookup(&'a self) -> &'a HotNode {
		match self {
			&HotRef::Hot(ref r) => &*r,
			&HotRef::Warm(ref r) => r,
		}
	}
}

// TODO can specialize these a bit
pub enum InsertResult {
	Ok,
	Flushed(Bucket, HotNode),
}

// enum NodeRefInt {
// 	Hot(RefCell<HotNode>),
// 	Warm(Rc<HotNode>),
// }

// TODO: this is the node itself, not a ref.
/// A reference to a node in a ready or hot state, together with information about
/// whether it's hot or cold. Used internally in a lot of places.
// /// Although its contents are static,
// /// NodeRef itself is lifetimed so it can be used internally in Handles, which need to pass
// /// a consistent lifetime parameter around.
pub enum NodeRef {
	// It's idiomatic to modify child nodes independenty of their parents in our code, so all child NodeStack
	// are wrapped in RefCells. One is free to BorrowMut such cells.
	// In particular, the futures implementation keeps a stack of mutable references around,
	// and you cannot borrow a thing and a member of that thing at the same time. So we must use pointers.
	// In the future, we might consider using raw pointers. For now, we rely on Rust.

	// TODO: in the future, don't use RC here.
	Hot(Rc<RefCell<HotNode>>),
	Warm(Rc<HotNode>),
}

impl NodeRef {
	/* Constructors */
	pub fn new(n: HotNode) -> Self {
		NodeRef::Hot(Rc::new(RefCell::new(n)))
	}

	// pub fn fork(&self) -> Self {
	// 	match self {
	// 		&NodeRef::Hot(_) => panic!("cannot fork a hot node"),
	// 		&NodeRef::Warm(ref rc) => NodeRef::Warm(rc.clone()),
	// 	}
	// }

	/* Accessors */
	pub fn apply<F, R> (&self, f: F) -> R where
	F: Fn(&HotNode) -> R
	{
		match self {
			// Same call, different objects. Necessary because of the monomorphism restriction.
			&NodeRef::Hot(ref w_rfc_hn) => f(w_rfc_hn.deref().borrow().deref()),
			&NodeRef::Warm(ref w_hn) => f(w_hn.deref()),
		}
	}

	pub fn handle(&self) -> NodeHandle {
		match self {
			&NodeRef::Hot(ref rc_) => NodeHandle::Hot(Rc::downgrade(&rc_)),
			&NodeRef::Warm(ref rc_) => NodeHandle::Warm(Rc::downgrade(&rc_)),
		}
	}

	/// Reassigns this node ref to the HotNode referred to by the given HotHandle,
	/// with the idea that the given HotRef is a modified version or copy of the contained Node.
	/// In debug mode, asserts that such is true. TODO: should not be pub.
	#[cfg(debug_assertions)]
	pub fn reassign(&mut self, nref: HotHandle) {
		match nref {
			HotHandle::Existing(hn_ref) => debug_assert!(ptr_eq(self.acquire().lookup(), hn_ref.deref())),
			HotHandle::New(nref, hn_box_cell) => {
				// TODO: assert if this assignment is invalid.
				debug_assert!(ptr_eq(self, nref.unwrap()));
				// TODO: can we do self = ?
				let mut replacement = NodeRef::Hot(hn_box_cell);
				mem::swap(&mut replacement, self)
			}
		}
	}

	#[cfg(not(debug_assertions))]
	pub fn reassign(&mut self, nref: HotHandle) {
		match nref {
			HotHandle::Existing(hn_ref) => debug_assert!(ptr_eq(self.as_ref(), hn_ref)),
			HotHandle::New(_, hn_box) => {
				// TODO: assert if this assignment is invalid.
				let replacement = NodeRef::Hot(hn_box.map(|x| RefCell::new(x)));
				mem::swap(&mut replacement, self)
			}
		}
	}

	/* Thermodynamics */
	fn is_hot(&self) -> bool {
		match self {
			&NodeRef::Hot(_) => true,
			&_ => false,
		}
	}

	/// Acquire an object that can directly reference the underlying warm node.
	/// Guaranteed to return quickly (if a lock conflict happens, it will panic or cause UB).
	// TODO: can we return the reference directly?
	fn acquire<'a>(&'a self) -> HotRef<'a> {
		match self {
			&NodeRef::Hot(ref box_cell_hn) => HotRef::Hot(box_cell_hn.as_ref().borrow()),
			&NodeRef::Warm(ref rc_hn) => HotRef::Warm(rc_hn.as_ref()),
		}
	}

	/// Returns a hot NodeRef which may be modified, together with a reference to that node. May return self.
	fn heat<'a>(&'a self) -> (HotHandle<'a>, bool) {
		match self {
			&NodeRef::Hot(ref box_cell_hn) => (HotHandle::existing(box_cell_hn.as_ref().borrow_mut()), false),
			&NodeRef::Warm(ref rc_hn) => (HotHandle::heated(self, (*rc_hn).fork()), true),
		}
	}

	// fn heat(self) -> (NodeRef, bool) {
	// 	match self {
	// 		NodeRef::Hot(box_hn) => NodeRef::Hot(box_hn),
	// 		NodeRef::Warm(rc_hn) => NodeRef::Hot(Box::new(rc_hn.as_ref().fork())),
	// 	}
	// }

	pub fn cool(&mut self) {
		panic!("not implemented")
		// match self {
		// 	&mut NodeRef::Hot(_) => {
		// 		let newself = self;
		// 		box_hn.as_mut().cool();∂ar
		// 		let mut newself = NodeRef::Warm(Rc::new(*box_hn));
		// 		mem::swap(&mut newself, self)
		// 	},
		// 	&mut NodeRef::Warm(_) => (),
		// }
	}

	// // For the edge case where head has 1 child.
	// pub fn disown_only_child(&mut self) -> NodePtr {
	// 	if self.bucket_count() != 0 || self.is_leaf() {
	// 		panic!("called disown_only_child when buckets are present")
	// 	}
	// 	let mut r = NodePtr::empty();
	// 	mem::swap(&mut r, &mut self.children[0]);
	// 	r
	// }

	// fn insert<'a, D: Datum>(&'a mut self, k: &[u8], v: &D) -> InsertResult<'a> {
	// 	let search = self.warm().find_bucket(k);

	// 	match search {
	// 		Ok(_) => InsertResult::Dup,
	// 		Err(idx) => if self.warm().is_leaf() {
	// 			self.heat().as_mut().insert_at(idx as u16, Bucket::new(k, v), None)
	// 		} else {
	// 			unsafe {
	// 				// warm_child borrows self mutably, and insert extends that borrow since it might return a hot reference to the child itself.
	// 				// Here we trick Rust into letting it compile.
	// 				let insert_result = self.warm().warm_child(idx as u16).insert_at(idx as u16, Bucket::new(k, v), None).unsafe_strip_lifetime();
	// 				match insert_result {
	// 					InsertResult::Good(hotref) => {
	// 						// let r: HotRef<'a> = self.heat();
	// 						// r.as_mut().reassign_child(idx as u16, hotref);
	// 						// InsertResult::Good(r)
	// 						InsertResult::Good(self.heat())
	// 					},
	// 					// The child was flushed and split into two children. We need to insert both.
	// 					InsertResult::Flushed(hotref1, split_bucket, node2) => {
	// 						let r: HotRef<'a> = self.heat();
	// 						r.as_mut().reassign_child(idx as u16, hotref1);
	// 						// This may cause us to flush, in turn.
	// 						r.as_mut().insert_at(idx as u16, split_bucket, Some(NodePtr::new(node2)))
	// 					}
	// 					InsertResult::Dup => InsertResult::Dup,
	// 				}
	// 			}
	// 		},
	// 	}
	// }

	// /// Like insert, but where this node is a root node.
	// pub fn insert_for_root<D: Datum>(mut self, k: &[u8], v: &D) -> Self {
	// 	unsafe {
	// 		let insert_result = self.insert(k, v).unsafe_strip_lifetime();
	// 		match insert_result {
	// 			InsertResult::Good(newnode) => {
	// 				self.reassign(newnode);
	// 				self
	// 			}
	// 			InsertResult::Flushed(hotref1, split_bucket, node2) => {
	// 				self.reassign(hotref1);
	// 				NodeRef::new(HotNode::new_from_two(NodePtr::wrap(self), split_bucket, NodePtr::new(node2)))
	// 			}
	// 			InsertResult::Dup => panic!("dup handling not implemented")
	// 		}
	// 	}
	// }

	// pub fn get(&self, k: &[u8]) -> Option<&ByteBox> {
	// 	match self.find_bucket(k) {
	// 		Ok(idx) => Some(&self.get_bucket(idx).v),
	// 		Err(idx) => {
	// 			if self.is_leaf() {
	// 				None
	// 			} else {
	// 				self.get_child(idx).get(k)
	// 			}
	// 		},
	// 	}
	// }

	// /// I think this ptr/raw node split is correct. Only a mut self can be heated.
	// /// No, it's wrong. We need to clone and heat a copy of ourselves.

	// /// Postcondition: May leave this node deficient.
	// pub fn delete(&mut self, k: &[u8]) -> bool {
	// 	// Unlike in insert, we rebalance *after* delete.
	// 	match self.find_bucket(k) {
	// 		Ok(idx) => {
	// 			if self.is_leaf() {
	// 				Some(Self::cool(self.to_hot().delete_bucket(idx)));
	// 				true
	// 		    } else {
	// 				if idx > 0 {
	// 					// get leftmost descendant from right child
	// 					let new_child = self.get_child(idx + 1).heat();
	// 			 	    let new_bucket = new_child.yank_leftmost_bucket();
	// 			 	    let hn = self.heat();
	// 			 	    hn.replace_bucket(idx, new_bucket);
	// 			 	    hn.replace_child(idx + 1, new_child);
	// 			 	    hn.check_deficient_child(idx + 1);
	// 			 	    true
	// 				} else {
	// 					// get rightmost descendant from left child
	// 			 	    let new_child = self.get_child(idx).heat();
	// 			 	    let new_bucket = new_child.yank_rightmost_bucket();
	// 			 	    r.replace_bucket(idx, new_bucket);
	// 			 	    r.replace_child(idx, new_child);
	// 			 	    r.check_deficient_child(idx);
	// 			 	    true
	// 			    }
	// 		    }
	// 		},
	// 		Err(idx) => if !self.is_leaf() {
	// 			match self.get_child_mut(idx).delete(k) {
	// 				Some(newchild) => {
	// 					let r = self.heat();
	// 					r.check_deficient_child(idx);
	// 					Some(Self::cool(r))
	// 				}
	// 				None => None
	// 			}
	// 		} else {
	// 			None
	// 		},
	// 	}
	// }

	/* Invariant checking */
	#[allow(dead_code)]
	fn quickcheck_invariants(&self) {
		self.acquire().lookup().check_invariants(None, None, false);
	}

	pub fn check_invariants(&self) {
		// TODO maybe some hot/warm invariants?
		self.acquire().lookup().check_invariants(None, None, true);
	}
}

// TODO: figure out a good r/w interface for packing/unpacking nodes.
// Packer/Unpacker<T>?
pub struct HotNode {
	// TODO
// pub struct HotNode<TC: TreeTypeclass> {
	/// Invariant: between (NODE_CAPACITY - 1) / 2 and NODE_CAPACITY - 1 unless we are the top node,
	/// in which case this is between 0 and NODE_CAPACITY - 1.
	/// When flushed, this is between 0 and NODE_CAPACITY - 2.
	bucket_count: u16,
	// count_bytes: u16,
	/// Buckets: key z pairs in this node.
	/// Invariant: the buckets in the interval [0, bucket_count) are populated,
	/// all others are not.
    buckets: [BucketPtr; NODE_CAPACITY as usize - 1],
	/// Invariant: If this is a leaf, all children are empty. Otherwise, child_count == bucket_count = 1.
    children: [NodePtr; NODE_CAPACITY as usize],
    // TODO
    // children: [Option<RefCell<NodeRef>>; NODE_CAPACITY as usize],
    // TODO
    // children: [TC:RefCellT<NodePtr::empty()>; NODE_CAPACITY as usize - 1];
}

impl HotNode {
	/* Constructors */
	pub fn empty() -> HotNode {
		unsafe {
			HotNode {
				// height: 0,
				bucket_count: 0,
				// We could use mem::uninitialized, but this is a test class.
        	    buckets: make_array!(|_| BucketPtr::empty(), (NODE_CAPACITY - 1) as usize),
       	    	children: make_array!(|_| NodePtr::empty(), NODE_CAPACITY as usize),
       	    	// children: make_array!(|_| None, NODE_CAPACITY as usize),
        	    // TODO
       	    	// children: make_array!(|_| TC::RefCellT<NodePtr::empty()>, NODE_CAPACITY as usize),
			}
		}
	}

	pub fn new_from_one<V: Datum>(k: &[u8], v: &V) -> HotNode {
		let mut r = Self::empty();
		r.buckets[0] = BucketPtr::new(k, v);
		r.bucket_count = 1;
		r
	}

	pub fn new_from_two(n1: NodePtr, b1: Bucket, n2: NodePtr) -> HotNode {
		let mut r = Self::empty();

		r.buckets[0] = BucketPtr::wrap(b1);
		r.children[0] = n1;
		r.children[1] = n2;
		r.bucket_count = 1;

		r
	}

	fn fork(&self) -> HotNode {
		panic!("not implemented")
	}

	// pub fn fork(&self) -> HotNode {
	// 	let mut r = Self::empty();
	// 	r.bucket_count = self.bucket_count();

	// 	for i in 0..(self.bucket_count() as usize) {
	// 		r.buckets[i] = self.buckets[i].clone();
	// 		r.children[i] = self.children[i].fork();
	// 	}

	// 	r
	// }

	/* Fast accessors */
	fn bucket_count(&self) -> u16 {
		self.bucket_count
	}

	fn child_count(&self) -> u16 {
		self.bucket_count() + 1
	}

	fn key(&self, idx: u16) -> &[u8] {
		self.buckets[idx as usize].key()
	}

	fn bucket_ptr(&self, idx: u16) -> &BucketPtr {
		&self.buckets[idx as usize]
	}

	fn child_ptr(&self, idx: u16) -> &NodePtr {
		&self.children[idx as usize]
	}

	fn child_ref(&self, idx: u16) -> &NodeRef {
		&self.children[idx as usize].unwrap()
	}

	// fn child_ref_mut(&self, idx: u16) -> &mut NodeRef {
	// 	// This is 'safe' in the sense it's safe to modify the child even if the parent is immutable,
	// 	// provided one observes invariants.
	// 	unsafe { mem::transmute::<&NodeRef, &mut NodeRef>(self.child_ref(idx)) }
	// }

	/* Thermodynamic accessors */
	fn warm_child(&self, idx: u16) -> &HotNode {
		panic!("this is wrong")
		// self.children[idx as usize].unwrap().acquire()
	}

	fn warm_value(&mut self, idx: u16) -> &[u8] {
		self.buckets[idx as usize].warm_value()
	}

	/* Thermodynamic mutators */
	// pub fn cool(&mut self) {
	// 	for i in 0..(self.child_count() as usize) {
	// 		self.children[i].cool()
	// 	}
	// }

	/* Basic helpers */
	pub fn is_leaf(&self) -> bool {
		self.children[0].is_empty()
	}

	/// A node is deficient iff it can be merged with another deficient node
	/// without needing a flush.
	fn is_deficient(&self) -> bool {
		self.bucket_count < (NODE_CAPACITY - 1) / 2
	}

	/* Insert helpers */

	/// Returns Ok(i) if bucket[i]'s key is equal to the given key.
	/// Returns Err(i) where bucket[i] has the first key greater than the given key.
	/// If buckets are empty (which probably shouldn't happen), returns Err(0).
	fn find_bucket(&self, k: &[u8]) -> Result<u16, u16> {
		// TODO: we can make this faster with a subslice.
		self.buckets[0..(self.bucket_count() as usize)]
		.binary_search_by(|bp| bp.v.as_ref().unwrap().k.bytes().cmp(k.bytes()))
		.map(|x| x as u16)
		.map_err(|x| x as u16)
	}

	fn needs_flush(&self) -> bool {
		self.bucket_count == NODE_CAPACITY - 1
	}

	/// Fully flushes this node, making it ready for insertion. May cause the node to split. Does not modify children.
	/// Flushing is the only operation allowed to create new nodes. In addition, this particular flush
	/// may not change the level of any node of the tree, so a fully balanced tree remains so.
	/// Preconditions: None.
	/// Result: This node is fully flushed.
	/// Returns: If this node was split, the bucket and node pointer that should be inserted into a parent node.
	/// Note that the bucket should be this node's new parent bucket, and the new node should inherit the old bucket.

	// TODO: figure out how not to copy HotNode. Box it?
	pub fn flush(&mut self) -> (Bucket, HotNode) {
		debug_assert!(self.needs_flush());

		let bucket_count = self.bucket_count as usize;
		// Split down the middle. If we have (2n + 1) buckets, this picks bucket n (0-indexed), exactly in the middle.
		// If we have 2n buckets the nodes will be uneven so we pick n, saving us one bucket copy.
		let split_idx = bucket_count / 2;
		let mut n2 = Self::empty();

		// TODO: use rotate fns
		for i in (split_idx + 1)..bucket_count {
			let dst_idx = i - split_idx - 1; // start from 0 in dst
			mem::swap(&mut self.buckets[i], &mut n2.buckets[dst_idx]);
			// Note that this is safe even if we are a leaf, because if so, all children are empty.
			mem::swap(&mut self.children[i], &mut n2.children[dst_idx]);
		}
		// Don't forget the last child
		mem::swap(&mut self.children[bucket_count], &mut n2.children[bucket_count - split_idx - 1]);

		n2.bucket_count = self.bucket_count - split_idx  as u16 - 1;
		self.bucket_count = split_idx as u16;

		// Now our children are divided among two nodes. This leaves an extra bucket, which we return
		// so the parent node can do something with it.
		let mut bp = BucketPtr::empty();
		mem::swap(&mut bp, &mut self.buckets[split_idx]);

		// We are done, time to return.
		// println!("split {} {} -> {} {}", bucket_count, split_idx, self.bucket_count, n2.bucket_count);

		// self.quickcheck_invariants();
		// n2.quickcheck_invariants();

		(bp.unwrap(), n2)
	}

	// TODO: can we avoid passing the new node on the stack?
	fn insert_unchecked(&mut self, idx: u16, b: Bucket, right_child: Option<NodePtr>) {
		rotate_in(&mut BucketPtr::wrap(b), &mut self.buckets[(idx as usize)..(self.bucket_count as usize + 1)]);
		match right_child {
			Some(mut nptr) => {
				// TODO: debug assertions about nptr
				rotate_in(&mut nptr, &mut self.children[(idx as usize + 1)..(self.bucket_count as usize + 2)]);
			}
			None => (),
		}
		self.bucket_count += 1;
	}

	/// Requirements: b.key is between bucket[idx].key and bucket[idx + 1].key, if the latter exists,
	/// or merely greater than bucket[idx].key if not.
	/// The values in right_child are *greater* than b.key, and less than bucket[idx + 1].key.
	fn insert_at(&mut self, idx: u16, b: Bucket, right_child: Option<NodePtr>) -> InsertResult {
		debug_assert!(self.is_leaf() == right_child.is_none());

		if self.needs_flush() {
			let (bp, mut node2) = self.flush();
			// Suppose ref1 now has 7 buckets and ref2 now has 7 buckets. This means bucket number 7 is now
			// a new parent bucket. So if idx is less than 7, we insert into ref1.
			let idx2 = idx - self.bucket_count() - 1;
			if idx2 >= 0 {
				node2.insert_unchecked(idx2, b, right_child);
			} else {
				self.insert_unchecked(idx, b, right_child);
			}
			InsertResult::Flushed(bp, node2)
		} else {
			self.insert_unchecked(idx, b, right_child);
			InsertResult::Ok
		}
	}

	fn reassign_child(&mut self, idx: u16, n: HotHandle) {
		self.children[idx as usize].unwrap_mut().reassign(n)
	}

	/* Get helpers */

	// /* Delete helpers */
	// fn merge_deficient_nodes(&mut self, right_other: &mut Self, bucket_to_steal: &mut BucketPtr) {
	// 	debug_assert!(self.is_deficient() && right_other.is_deficient());
	// 	let bucket_count = self.bucket_count as usize;
	// 	let other_bucket_count = right_other.bucket_count as usize;

	// 	// We are 'stealing' this bucket from the parent
	// 	mem::swap(&mut self.buckets[bucket_count], bucket_to_steal);
	// 	swap(
	// 		&mut self.buckets[(bucket_count + 1)..(bucket_count + other_bucket_count + 1)],
	// 		&mut right_other.buckets[..other_bucket_count]);
	// 	swap(
	// 		&mut self.children[(bucket_count + 1)..(bucket_count + other_bucket_count + 2)],
	// 		&mut right_other.children[..(other_bucket_count + 1)]);
	// 	self.bucket_count += right_other.bucket_count + 1;

	// 	// self.check_invariants();
	// }

	// fn borrow_left(&mut self, left: &mut NodePtr, left_parent_bucket: &mut BucketPtr) {
	// 	let mut ln = left.unwrap_hot();
	// 	let num_to_borrow = (ln.bucket_count - self.bucket_count + 1) / 2;
	// 	debug_assert!(num_to_borrow >= 1, "invalid bucket sizes: {} {}", ln.bucket_count, self.bucket_count);
	// 	let bucket_count = self.bucket_count as usize;

	// 	// Make room for the things we're borrowing.
	// 	rotate_right(&mut self.buckets[..(bucket_count + num_to_borrow as usize)], num_to_borrow as usize);
	// 	rotate_right(&mut self.children[..(bucket_count + num_to_borrow as usize + 1)], num_to_borrow as usize);

	// 	// The first bororwed bucket becomes the new parent bucket, and the old parent bucket is moved into this node.
	// 	mem::swap(left_parent_bucket, &mut self.buckets[num_to_borrow as usize - 1]);
	// 	mem::swap(&mut ln.buckets[(ln.bucket_count - num_to_borrow) as usize], left_parent_bucket);

	// 	// Swap in the buckets and children. Empty buckets will be swapped into ln.
	// 	// We borrow n children and n-1 buckets (additionally, we did the little shuffle above, borrowing n total).
	// 	swap(
	// 		&mut ln.buckets[(ln.bucket_count - num_to_borrow + 1) as usize..ln.bucket_count as usize],
	// 		&mut self.buckets[..(num_to_borrow as usize - 1)]);
	// 	swap(
	// 		&mut ln.children[(ln.bucket_count - num_to_borrow + 1) as usize..(ln.bucket_count as usize + 1)],
	// 		&mut self.children[..(num_to_borrow as usize)]);

	// 	self.bucket_count += num_to_borrow;
	// 	ln.bucket_count -= num_to_borrow;

	// 	// self.quickcheck_invariants();
	// 	// ln.quickcheck_invariants();
	// 		// self.check_invariants();
	// 		// ln.check_invariants();
	// }

	// fn borrow_right(&mut self, right: &mut NodePtr, right_parent_bucket: &mut BucketPtr) {
	// 	let mut rn = right.unwrap_hot();
	// 	let num_to_borrow = (rn.bucket_count - self.bucket_count + 1) / 2;
	// 	debug_assert!(num_to_borrow >= 1, "invalid bucket sizes: {} {}", self.bucket_count, rn.bucket_count);
	// 	let bucket_count = self.bucket_count as usize;

	// 	// The last bororwed bucket becomes the new parent bucket, and the old parent bucket is moved into this node.
	// 	mem::swap(right_parent_bucket, &mut self.buckets[bucket_count]);
	// 	mem::swap(&mut rn.buckets[num_to_borrow as usize - 1], right_parent_bucket);

	// 	// Swap in the buckets and children. Empty buckets will be swapped into rn.
	// 	// We borrow n children and n-1 buckets (additionally, we did the little shuffle above, borrowing n total).
	// 	swap(
	// 		&mut rn.buckets[0..(num_to_borrow - 1) as usize],
	// 		&mut self.buckets[(bucket_count + 1)..(bucket_count + num_to_borrow as usize)]);
	// 	swap(
	// 		&mut rn.children[0..num_to_borrow as usize],
	// 		&mut self.children[(bucket_count + 1)..(bucket_count + num_to_borrow as usize + 1)]);

	// 	// Adjust the positions in the right node.
	// 	rotate_left(&mut rn.buckets[0..(rn.bucket_count ) as usize], num_to_borrow as usize);
	// 	rotate_left(&mut rn.children[0..(rn.bucket_count + 1) as usize], num_to_borrow as usize);

	// 	self.bucket_count += num_to_borrow;
	// 	rn.bucket_count -= num_to_borrow;

	// 	// self.quickcheck_invariants();
	// 	// rn.quickcheck_invariants();
	// 		// self.check_invariants();
	// 		// rn.check_invariants();
	// }

	// /// Preconditions: The parent node, if it exists, is not deficient.
	// /// (It may become deficient as a result of this merge.) As such,
	// /// at least one neighbor is guaranteed.
	// /// After this is run, one bucket pointer may be empty. If so, the parent must delete
	// /// that bucket.

	// // TODO: ptrs and bucketptrs are already options...
	// fn merge(&mut self, left: Option<&mut NodePtr>, left_parent_bucket: Option<&mut BucketPtr>,
	// 	right: Option<&mut NodePtr>, right_parent_bucket: Option<&mut BucketPtr>)
	// -> Option<bool> {
	// 	debug_assert!(self.is_deficient());
	// 	debug_assert!(left.is_some() || right.is_some());
	// 	debug_assert!(left.is_some() == left_parent_bucket.is_some());

	// 	match left {
	// 		Some(lv) => {
	// 			if lv.unwrap().is_deficient() {
	// 				lv.unwrap_hot().merge_deficient_nodes(self, left_parent_bucket.unwrap());
	// 				Some(false)
	// 			} else {
	// 				match right {
	// 					Some(rv) => {
	// 						if rv.unwrap().is_deficient() {
	// 							self.merge_deficient_nodes(rv.unwrap_hot(), right_parent_bucket.unwrap());
	// 							Some(true)
	// 						} else if rv.unwrap().bucket_count > lv.unwrap().bucket_count {
	// 							self.borrow_right(rv, right_parent_bucket.unwrap());
	// 							None
	// 						} else {
	// 							self.borrow_left(lv, left_parent_bucket.unwrap());
	// 							None
	// 						}
	// 					}
	// 					None => {
	// 						self.borrow_left(lv, left_parent_bucket.unwrap());
	// 					    None
	// 					}
	// 				}
	// 			}
	// 		}
	// 		None => {
	// 			if right.as_ref().unwrap().unwrap().is_deficient() {
	// 				self.merge_deficient_nodes(right.unwrap().unwrap_hot(), right_parent_bucket.unwrap());
	// 				Some(true)
	// 			} else {
	// 				self.borrow_right(right.unwrap(), right_parent_bucket.unwrap());
	// 				None
	// 			}
	// 		}
	// 	}
	// }

	// /// Like split_at_mut but with reasonable defaults for out of bounds indices.
	// fn split_or_empty<T>(t: &mut [T], idx: isize) -> (&mut [T], &mut [T]) {
	// 	if idx >= 0 {
	// 		if (idx as usize) < t.len() {
	// 			t.split_at_mut(idx as usize)
	// 		} else {
	// 			(t, [].as_mut())
	// 		}
	// 	} else {
	// 		([].as_mut(), t)
	// 	}
	// }

	// /// Returns true if merging happened (and hence buckets[idx] has changed).
	// // TODO: this code is pure shit.
	// fn check_deficient_child(&mut self, idx: usize) {
	// 	debug_assert!(idx <= self.bucket_count as usize);

	// 	if self.get_child(idx).is_deficient() {
	// 		let delete_result;
	// 		let bucket_count = self.bucket_count as usize;

	// 		{
	// 			// Get the borrows we need, in a way that won't make Rust freak out.
	// 			// (A borrow by index borrows the whole array.)

	// 			// If the idx is n, left child and right child are at n - 1 and n + 1.
	// 			// Left bucket and right bucket are at n - 1 and n.
	// 			let (left_bucket_dummy, right_bucket_dummy) = Self::split_or_empty(&mut self.buckets, idx as isize);
	// 			let (left_child_dummy, mut mid_and_right_child_dummy) = Self::split_or_empty(&mut self.children, idx as isize);
	// 			let (mid_child_dummy, right_child_dummy) = Self::split_or_empty(&mut mid_and_right_child_dummy, 1);

	// 			// Now we have the borrows we need, we can start bundling our arguments.
	// 			// TODO: ptrs are already options...
	// 			let left_sibling;
	// 			let left_parent_bucket;
	// 			let right_sibling;
	// 			let right_parent_bucket;
	// 			let child = mid_child_dummy[0].unwrap_mut().heat();

	// 			if idx > 0 {
	// 				left_sibling = Some(&mut left_child_dummy[idx - 1]);
	// 				left_parent_bucket = Some(&mut left_bucket_dummy[idx - 1]);
	// 			} else {
	// 				left_sibling = None;
	// 				left_parent_bucket = None;
	// 			}

	// 			if idx < bucket_count {
	// 				right_sibling = Some(&mut right_child_dummy[0]);
	// 				right_parent_bucket = Some(&mut right_bucket_dummy[0]);
	// 			} else {
	// 				right_sibling = None;
	// 				right_parent_bucket = None;
	// 			}

	// 			// A lot of syntax nonsense for one call!
	// 			delete_result = child.merge(left_sibling, left_parent_bucket, right_sibling, right_parent_bucket);
	// 			mid_child_dummy[0] = NodePtr::cool(child);
	// 		}

	// 		match delete_result {
	// 			// false == left bucket, middle child was deleted
	// 			Some(false) => {
	// 				rotate_out(&mut self.buckets[(idx - 1)..bucket_count], &mut BucketPtr::empty());
	// 				rotate_out(&mut self.children[idx..(bucket_count + 1)], &mut NodePtr::empty());
	// 				self.bucket_count -= 1;
	// 			}
	// 			// true = right bucket, right sibling was deleted
	// 			Some(true) => {
	// 				rotate_out(&mut self.buckets[idx..bucket_count], &mut BucketPtr::empty());
	// 				rotate_out(&mut self.children[(idx + 1)..(bucket_count + 1)], &mut NodePtr::empty());
	// 				self.bucket_count -= 1;
	// 			}
	// 			None => (),
	// 		}

	// 		// self.quickcheck_invariants();
	// 		// self.check_invariants();
	// 	}
	// }

	// /// Postcondition: May leave this node deficient. Will not leave descendant nodes deficient.
	// fn yank_rightmost_bucket(&mut self) -> BucketPtr {
	// 	let bucket_count = self.bucket_count as usize;

	// 	if self.is_leaf() {
	// 		let mut r = BucketPtr::empty();
	// 		mem::swap(&mut r, &mut self.buckets[bucket_count - 1]);
	// 		self.bucket_count -= 1;
	// 		r
	// 	} else {
	// 		let r = self.doto_child(bucket_count, Self::yank_rightmost_bucket);
	// 		self.check_deficient_child(bucket_count);
	// 		r
	// 	}
	// }

	// /// Postcondition: May leave this node deficient. Will not leave descendant nodes deficient.
	// fn yank_leftmost_bucket(&mut self) -> BucketPtr {
	// 	if self.is_leaf() {
	// 		let mut r = BucketPtr::empty();
	// 		rotate_out(&mut self.buckets[0..self.bucket_count as usize], &mut r);
	// 		self.bucket_count -= 1;
	// 		r
	// 	} else {
	// 		let r = self.doto_child(0, Self::yank_leftmost_bucket);
	// 		self.check_deficient_child(0);
	// 		r
	// 	}
	// }

	// fn delete_bucket(&mut self, idx: usize) -> () {
	// 	rotate_out(&mut self.buckets[idx..self.bucket_count as usize], &mut BucketPtr::empty());
	// 	self.bucket_count -= 1;
	// }

	/* Invariants */

	fn check_invariants(&self, parent_lower_bound: Option<&[u8]>, parent_upper_bound: Option<&[u8]>, recurse: bool) {
		// TODO: validate all leaves are at the same level

		// Validate the bucket count
		// for i in 0..(NODE_CAPACITY - 1) {
		// 	if i >= self.bucket_count() {
		// 		assert!(self.bucket_ref(i).is_empty(), "expected empty bucket in position {}", i);
		// 	} else {
		// 		assert!(!self.bucket_ref(i).is_empty(), "expected populated bucket in position {}", i);
		// 		// Validate sorted order
		// 		if i > 1 {
		// 			assert!(self.key(i) > self.key(i - 1));
		// 		}
		// 	}
		// }

		// // Validate bounds
		// assert!(parent_lower_bound.is_none() || self.key(0) > parent_lower_bound.unwrap());
		// assert!(parent_upper_bound.is_none() || self.key(self.bucket_count() - 1) < parent_upper_bound.unwrap());

		// // TODO: assert non-head nodes are never deficient.
		// assert!(self.is_leaf() || self.bucket_count() >= 1);

		// // Validate the children
		// for i in 0..NODE_CAPACITY {
		// 	if self.is_leaf() || i >= self.bucket_count() + 1 {
		// 		assert!(self.child_ref(i).is_empty());
		// 	} else {
		// 		assert!(!self.child_ref(i).is_empty());

		// 		if recurse {
		// 			let lower_bound;
		// 			if i == 0 {
		// 				lower_bound = None;
		// 			} else {
		// 				lower_bound = Some(self.key(i - 1));
		// 			}

		// 			let upper_bound;
		// 			if i == self.bucket_count() {
		// 				upper_bound = None;
		// 			} else {
		// 				upper_bound = Some(self.key(i));
		// 			}

		// 			self.warm_child(i).check_invariants(lower_bound, upper_bound, recurse);
		// 		}
		// 	}
		// }
	}
}

// A simple in-memory persistent b-tree.
pub struct PersistentBTree {
	head: NodePtr,
}

impl PersistentBTree {
	pub fn new() -> PersistentBTree {
		PersistentBTree {
			head: NodePtr::empty(),
		}
	}
}

impl ByteMap for PersistentBTree {
	type D = ByteBox;

	fn insert<K: Key + ?Sized, V: Datum>(&mut self, k: &K, v: &V) -> () {
		// Dummy to make the compiler behave. Since we're dealing in Options and Boxes, shouldn't have a runtime cost.
		let mut dummy = NodePtr::empty();
		mem::swap(&mut dummy, &mut self.head);
		self.head = dummy.insert(k.bytes(), v);
	}

	fn get<K: Key + ?Sized>(&mut self, k: &K) -> Option<&Self::D> {
		panic!("not implemented")
	}

	fn delete<K: Key + ?Sized>(&mut self, k: &K) -> bool {
		panic!("not implemented")
	}

	// fn get<K: Key + ?Sized>(&mut self, k: &K) -> Option<&Self::D> {
	// 	self.head.get_for_root(k.bytes())
	// }

	// fn delete<K: Key + ?Sized>(&mut self, k: &K) -> bool {
	// 	let mut dummy = NodePtr::empty();
	// 	mem::swap(&mut dummy, &mut self.head);
	// 	let (newhead, r) = dummy.delete_for_root(k.bytes());
	// 	self.head = newhead;
	// 	r
	// }

	fn check_invariants(&self) {
		self.head.check_invariants();
	}
}
