//! memnode.rs
//!
//! An in-memory, modifiable node.

use std::cell::{RefCell, RefMut};
use std::rc::Rc;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::ptr;

use data::{ByteRc, Datum, Key};

use tree::bucket::*;
use tree::counter::*;
use tree::noderef::*;
use tree::util::*;

/// TODO deftype for idx's

/// The max capacity of a MemNode.
const NODE_CAPACITY: u16 = 16;

/// A simple pointer used internally by MemNode.
/// This class was historically introduced because of over-strong coupling between
/// MemNode's internals and client classes. Now, it's a simple Option,
/// but we leave it around.
///
/// We could replace this with a humble value (either in debug or all configs), but we find that
/// unlikely to yield major perf improvements. But who knows.
struct MemPtr<T> {
    v: Option<T>,
}

impl<T> MemPtr<T> {
    pub fn empty() -> Self {
        MemPtr {
            v: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.v.is_none()
    }

    pub fn wrap(t: T) -> Self {
        MemPtr { v: Some(t), }
    }

    // TODO: can we avoid this? clone should work just as good for referent types
    pub fn unwrap(self) -> T {
    	self.v.unwrap()
    }
}

impl<T> Deref for MemPtr<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.v.as_ref().unwrap()
	}
}

impl<T> DerefMut for MemPtr<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.v.as_mut().unwrap()
	}
}

impl<T> Clone for MemPtr<T> where T: Clone {
	fn clone(&self) -> Self {
		MemPtr {
			v: self.v.clone(),
		}
	}
}

// TODO: the heirarchy: disk has no deps on hot

// TODO can specialize these a bit
/// The result of inserting into a MemNode. Occasionally, nodes need to "split"
/// into separate nodes.
pub enum InsertResult {
	Ok,
	Flushed(Bucket, MemNode),
}

/// A handle to a hot node which can be quickly dereferenced. Note that it's lifetimed--
/// HotHandles are intended to be ephemeral.
pub enum HotHandle<'a> {
	Existing(RefMut<'a, MemNode>),
	// The NodeRef is used for debug assertions. It is forbidden to reassign a NodeRef to a HotHandle
	// that did not 'come from' that NodeRef.
	// We use a Rc<RefCell> here so it's easier to pass into NodeRef without copying.
	// Becaues RCs are thin pointers to a {refcount, T} pair, this has very little performance penalty
	// and saves us a copy when we 'cool' a MemNode.
	New(Option<&'a FatNodeRef>, Rc<RefCell<MemNode>>),
}

impl<'a> HotHandle<'a> {
	/// Do something to the referenced MemNode.
	pub fn apply_mut<F, R> (&mut self, f: F) -> R where F: FnOnce(&mut MemNode) -> R
	{
		match self {
			// Same call, different objects. Necessary because of the monomorphism restriction.
			&mut HotHandle::Existing(ref mut rfm_hn) => f(rfm_hn.deref_mut()),
			&mut HotHandle::New(_, ref mut rc_rfc_hn) => f(Rc::get_mut(rc_rfc_hn).unwrap().borrow_mut().deref_mut()),
		}
	}
}

pub struct MemNode {
	/// Invariant: between (NODE_CAPACITY - 1) / 2 and NODE_CAPACITY - 1 unless we are the top node,
	/// in which case this is between 0 and NODE_CAPACITY - 1.
	/// When split, this is between 0 and NODE_CAPACITY - 2.
	bucket_count: u16,
	/// Buckets: key pairs in this node.
	/// Invariant: the buckets in the interval [0, bucket_count) are populated,
	/// all others are not.
	// TODO: We don't need to use nullable pointers; we can use uninitialized data instead.
    buckets: [MemPtr<Bucket>; NODE_CAPACITY as usize - 1],
	/// Invariant: If this is a leaf, all children are empty. Otherwise, child_count == bucket_count = 1.
    children: [MemPtr<FatNodeRef>; NODE_CAPACITY as usize],
}

// TODO: rename MemNode -> MemNode
impl MemNode {
	/* Constructors */
	pub fn empty() -> MemNode {
		unsafe {
			MemNode {
				// height: 0,
				bucket_count: 0,
				// using mem::uninitialized might be faster
        	    buckets: make_array!(|_| MemPtr::empty(), (NODE_CAPACITY - 1) as usize),
       	    	children: make_array!(|_| MemPtr::empty(), NODE_CAPACITY as usize),
       	    	// children: make_array!(|_| None, NODE_CAPACITY as usize),
        	    // TODO
       	    	// children: make_array!(|_| TC::RefCellT<NodePtr::empty()>, NODE_CAPACITY as usize),
			}
		}
	}

	pub fn new_from_one(b: Bucket) -> MemNode {
		let mut r = Self::empty();

		r.buckets[0] = MemPtr::wrap(b);
		r.bucket_count = 1;

		r
	}

	pub fn new_from_two(n1: FatNodeRef, b1: Bucket, n2: FatNodeRef) -> MemNode {
		let mut r = Self::empty();

		r.buckets[0] = MemPtr::wrap(b1);
		r.children[0] = MemPtr::wrap(n1);
		r.children[1] = MemPtr::wrap(n2);
		r.bucket_count = 1;

		r
	}

	/// Immutes this MemNode, recursively immuting its children.
	pub fn cool(&mut self, txid: Counter) {
		for i in 0..self.child_count() as usize {
			self.children[i].cool(txid);
		}
	}

	/// Creates a copy of this MemNode. For this to make sense, the current node must be immutable.
	pub fn fork(&self) -> MemNode {
		// Right now, this is a poor man's Clone.
		let mut r = Self::empty();

		r.bucket_count = self.bucket_count;

		for i in 0..self.bucket_count() as usize {
			r.buckets[i] = self.buckets[i].clone();
		}

		for i in 0..self.child_count() as usize {
			r.children[i] = MemPtr::wrap(self.children[i].shallow_clone());
		}

		r
	}

	/* Fast accessors */
	fn bucket_count(&self) -> u16 {
		self.bucket_count
	}

	fn child_count(&self) -> u16 {
		if self.is_leaf() {
			0
		} else {
			self.bucket_count + 1
		}
	}

	fn bucket_ptr(&self, idx: u16) -> &MemPtr<Bucket> {
		&self.buckets[idx as usize]
	}

	fn key(&self, idx: u16) -> &[u8] {
		self.bucket_ptr(idx).deref().k.bytes()
	}

	/// Gets the value associated at a particular index.
	// TODO: should this be generic on node?
	pub fn value(&self, idx: u16) -> &ByteRc {
		&self.bucket_ptr(idx).deref().v
	}

	// fn bucket_ptr(&self, idx: u16) -> &BucketPtr {
	// 	&self.buckets[idx as usize]
	// }

	// fn child_ptr(&self, idx: u16) -> &NodePtr {
	// 	&self.children[idx as usize]
	// }

	fn child_ptr(&self, idx: u16) -> &MemPtr<FatNodeRef> {
		&self.children[idx as usize]
	}

	// TODO: return weak instead
	pub fn child_ref(&self, idx: u16) -> &FatNodeRef {
		self.child_ptr(idx).deref()
	}

	/* Basic helpers */
	pub fn is_leaf(&self) -> bool {
		self.children[0].is_empty()
	}

	// /// A node is deficient iff it can be merged with another deficient node
	// /// without needing a flush.
	// fn is_deficient(&self) -> bool {
	// 	self.bucket_count < (NODE_CAPACITY - 1) / 2
	// }

	/* Insert helpers */

	/// Returns Ok(i) if bucket[i]'s key is equal to the given key.
	/// Returns Err(i) where bucket[i] has the first key greater than the given key.
	/// If buckets are empty (which probably shouldn't happen), returns Err(0).
	pub fn find(&self, k: &[u8]) -> Result<u16, u16> {
		// TODO: we can make this faster with a subslice.
		self.buckets[0..(self.bucket_count() as usize)]
		.binary_search_by(|bp| bp.deref().k.bytes().cmp(k))
		.map(|x| x as u16)
		.map_err(|x| x as u16)
	}

	fn needs_split(&self) -> bool {
		self.bucket_count == NODE_CAPACITY - 1
	}

	/// Splits this node, making it ready for insertion. Does not modify children.
	/// Flushing is the only operation allowed to create new nodes. In addition, this particular split
	/// may not change the level of any node of the tree, so a fully balanced tree remains so.
	/// Preconditions: None.
	/// Result: This node is split.
	/// Returns: If this node was split, the bucket and node pointer that should be inserted into a parent node.
	/// Note that the bucket should be this node's new parent bucket, and the new node should inherit the old bucket.

	// TODO: figure out how not to copy MemNode. Box it?
	pub fn split(&mut self) -> (Bucket, MemNode) {
		debug_assert!(self.needs_split());

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
		let mut bp = MemPtr::empty();
		mem::swap(&mut bp, &mut self.buckets[split_idx]);

		// We are done, time to return.
		// println!("split {} {} -> {} {}", bucket_count, split_idx, self.bucket_count, n2.bucket_count);

		// self.quickcheck_invariants();
		// n2.quickcheck_invariants();

		(bp.unwrap(), n2)
	}

	// TODO: can we avoid passing the new node on the stack?
	fn insert_unchecked(&mut self, idx: u16, b: Bucket, right_child: Option<FatNodeRef>) {
		rotate_in(&mut MemPtr::wrap(b), &mut self.buckets[(idx as usize)..(self.bucket_count as usize + 1)]);
		match right_child {
			Some(mut nptr) => {
				// TODO: debug assertions about nptr
				rotate_in(&mut MemPtr::wrap(nptr),
					&mut self.children[(idx as usize + 1)..(self.bucket_count as usize + 2)]);
			}
			None => (),
		}
		self.bucket_count += 1;
	}

	/// Requirements: b.key is between bucket[idx].key and bucket[idx + 1].key, if the latter exists,
	/// or merely greater than bucket[idx].key if not.
	/// The values in right_child are *greater* than b.key, and less than bucket[idx + 1].key.
	// TODO: FatNodeRef shouldnt be pub
	pub fn insert_at(&mut self, idx: u16, b: Bucket, right_child: Option<FatNodeRef>) -> InsertResult {
		debug_assert!(self.is_leaf() == right_child.is_none());

		if self.needs_split() {
			let (bp, mut node2) = self.split();
			// Suppose ref1 now has 7 buckets and ref2 now has 7 buckets. This means bucket number 7 is now
			// a new parent bucket. So if idx is less than 7, we insert into ref1.
			let idx2 = idx as i32 - self.bucket_count() as i32 - 1;
			if idx2 >= 0 {
				node2.insert_unchecked(idx2 as u16, b, right_child);
			} else {
				self.insert_unchecked(idx, b, right_child);
			}
			InsertResult::Flushed(bp, node2)
		} else {
			self.insert_unchecked(idx, b, right_child);
			InsertResult::Ok
		}
	}

	pub fn reassign_child(&mut self, idx: u16, n: HotHandle) {
		self.children[idx as usize].deref_mut().reassign(n)
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
	pub fn check_invariants_helper(&self, parent_lower_bound: Option<&[u8]>, parent_upper_bound: Option<&[u8]>,
		is_hot: bool, recurse: bool) {
		// TODO: validate all leaves are at the same level

		// Validate the bucket count
		for i in 0..(NODE_CAPACITY - 1) {
			if i >= self.bucket_count() {
				assert!(self.bucket_ptr(i).is_empty(), "expected empty bucket in position {}", i);
			} else {
				assert!(!self.bucket_ptr(i).is_empty(), "expected populated bucket in position {}", i);
				// Validate sorted order
				if i > 1 {
					assert!(self.key(i) > self.key(i - 1));
				}
			}
		}

		// Validate bounds
		assert!(parent_lower_bound.is_none() || self.key(0) > parent_lower_bound.unwrap());
		assert!(parent_upper_bound.is_none() || self.key(self.bucket_count() - 1) < parent_upper_bound.unwrap());

		// TODO: assert non-head nodes are never deficient.
		assert!(self.is_leaf() || self.bucket_count() >= 1);

		// Validate the children
		for i in 0..NODE_CAPACITY {
			if self.is_leaf() || i >= self.bucket_count() + 1 {
				assert!(self.child_ptr(i).is_empty());
			} else {
				assert!(!self.child_ptr(i).is_empty());

				if recurse {
					let lower_bound;
					if i == 0 {
						lower_bound = None;
					} else {
						lower_bound = Some(self.key(i - 1));
					}

					let upper_bound;
					if i == self.bucket_count() {
						upper_bound = None;
					} else {
						upper_bound = Some(self.key(i));
					}

					self.child_ref(i).apply(|n| n.check_invariants_helper(lower_bound, upper_bound, is_hot, recurse));
				}
			}
		}
	}
}
