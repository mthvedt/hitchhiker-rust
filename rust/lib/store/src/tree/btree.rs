use std::borrow::Borrow;
use std::cmp::{Ord, Ordering};
use std::convert::TryFrom;
use std::mem;
use std::ptr;

use data::*;
use data::slice::ByteBox;

// A brain-dead b-tree for testing/comparison.

// TODO: make it a real life b tree.

struct NodePtr {
	v: Option<Box<Node>>,
}

impl NodePtr {
	fn empty() -> NodePtr {
		NodePtr {
			v: None,
		}
	}

	fn set(&mut self, n: Node) {
		self.v = Some(Box::new(n));
	}
}

struct ValuePtr {
	v: Option<ByteBox>,
}

impl ValuePtr {
	fn empty() -> ValuePtr {
		ValuePtr {
			v: None
		}
	}

	fn set(&mut self, v: ByteBox) {
		self.v = Some(v);
	}
}

const NODE_CAPACITY: u16 = 16;

struct Node {
    vals: [ValuePtr; NODE_CAPACITY as usize],
    children: [NodePtr; NODE_CAPACITY as usize],
}

impl Node {
	fn empty() -> Node {
		unsafe {
			Node {
        	    vals: make_array!(|_| ValuePtr::empty(), NODE_CAPACITY as usize),
       	    	children: make_array!(|_| NodePtr::empty(), NODE_CAPACITY as usize),
			}
		}
	}

	fn get_child(&mut self, nibble: u8) -> Option<&mut Node> {
		// TODO safety check in debug mode
		let mut ptr = &mut self.children[nibble as usize];
		match ptr.v {
			Some(ref mut v) => Some(v),
			None => None,
		}
	}

	fn get_or_create_child(&mut self, nibble: u8) -> &mut Node {
		// TODO safety check in debug mode
		let mut ptr = &mut self.children[nibble as usize];
		match ptr.v {
			Some(ref mut v) => v,
			None => {
				ptr.set(Self::empty());
				ptr.v.as_mut().unwrap() // now safe
			}
		}
	}

	fn get_ptr_for_hi_nibble<B, I>(&mut self, b: u8, k: I) -> Option<&mut ValuePtr> where
	B: Borrow<u8>,
	I: Iterator<Item = B>,
	{
		let n1 = (b & 0xf0) >> 4;
		let n2 = b & 0x0f;
		match self.get_child(n1) {
			Some(child) => child.get_ptr_for_lo_nibble(n2, k),
			None => None,
		}
	}

	fn get_ptr_for_lo_nibble<B, I>(&mut self, nibble: u8, mut k: I) -> Option<&mut ValuePtr> where
	B: Borrow<u8>,
	I: Iterator<Item = B>,
	{
		match k.next() {
			Some(bb) => {
				let b = bb.borrow().clone();
				match self.get_child(nibble) {
					Some(child) => child.get_ptr_for_hi_nibble(b, k),
					None => None,
				}
			},
			None => Some(&mut self.vals[nibble as usize]),
		}
	}

	fn get_ptr<B, I>(&mut self, mut k: I) -> Option<&mut ValuePtr> where
	B: Borrow<u8>,
	I: Iterator<Item = B>,
	{
		match k.next() {
			Some(b) => self.get_ptr_for_hi_nibble(b.borrow().clone(), k),
			None => panic!("Tried to get with empty key"), // TODO handle
		}
	}

	fn insert_for_hi_nibble<D, B, I>(&mut self, b: u8, k: I, v: &D) where
	D: Datum,
	B: Borrow<u8>,
	I: Iterator<Item = B>,
	{
		let n1 = (b & 0xf0) >> 4;
		let n2 = b & 0x0f;
		let mut child = self.get_or_create_child(n1);
		child.insert_for_lo_nibble(n2, k, v);
	}

	fn insert_for_lo_nibble<D, B, I>(&mut self, nibble: u8, mut k: I, v: &D) where
	D: Datum,
	B: Borrow<u8>,
	I: Iterator<Item = B>,
	{
		match k.next() {
			Some(b) => {
				let mut child = self.get_or_create_child(nibble);
				child.insert_for_hi_nibble(b.borrow().clone(), k, v);
			},
			None => self.finish_insert(nibble, v),
		}
	}

	fn finish_insert<D>(&mut self, nibble: u8, v: &D) where
	D: Datum,
	{
		// TODO handle errors
		self.vals[nibble as usize].set(ByteBox::from_value(v));
	}

	// Interface functions
	fn insert<D, B, I>(&mut self, mut k: I, v: &D) where
	D: Datum,
	B: Borrow<u8>,
	I: Iterator<Item = B>,
	{
		match k.next() {
			Some(b) => self.insert_for_hi_nibble(b.borrow().clone(), k, v),
			None => panic!("Tried to insert with empty key"), // TODO handle
		}
	}

	fn get<B, I>(&mut self, k: I) -> Option<&ByteBox> where
	B: Borrow<u8>,
	I: Iterator<Item = B>,
	{
		match self.get_ptr(k) {
			Some(vptr) => match vptr.v {
				Some(ref mut val) => Some(val),
				None => None,
			},
			None => None,
		}
	}

	// Interface functions
	fn delete<B, I>(&mut self, k: I) -> bool where
	B: Borrow<u8>,
	I: Iterator<Item = B>,
	{
		match self.get_ptr(k) {
			Some(mut vptr) => {
				vptr.v = None;
				true
			},
			None => false,
		}
	}
}

// TODO: Key and StackDatum.
// TODO: move to module level doc the below.
/// A key is anything that can be (quickly, efficiently) converted to a byte iterator.
/// It is the same as, but more broadly implemented than, IntoIterator<[u8]>. Though it is passed
/// by value, most impls will be references.
/// A value is a Datum, a set of bytes that can be streamed. It should be passed by reference.
pub trait ByteMap {
	type D: Datum;

	/// Note that we only accept references that can be quickly converted to keys and values,
	/// for performance reasons.
	fn insert<K: Key + ?Sized, V: Datum>(&mut self, k: &K, v: &V) -> ();

	/// This is mutable because gets may introduce read conflicts, and hence mutate the underlying datastructure.
	fn get<K: Key + ?Sized>(&mut self, k: &K) -> Option<&Self::D>;

	fn delete<K: Key + ?Sized>(&mut self, k: &K) -> bool;

	fn check_invariants(&self);
}

// pub trait ByteTree: ByteMap {

// }

// TODO: rename. 'stupid btree'? We will delete this eventually.
pub struct BTree {
	head: Node,
}

impl BTree {
	pub fn new() -> BTree {
		BTree {
			head: Node::empty(),
		}
	}
}

impl ByteMap for BTree {
	type D = ByteBox;

	fn insert<K: Key + ?Sized, V: Datum>(&mut self, k: &K, v: &V) -> () {
		self.head.insert(k.bytes().iter(), v);
	}

	fn get<K: Key + ?Sized>(&mut self, k: &K) -> Option<&Self::D> {
		self.head.get(k.bytes().iter())
	}

	fn delete<K: Key + ?Sized>(&mut self, k: &K) -> bool {
		self.head.delete(k.bytes().iter())
	}

	fn check_invariants(&self) {
		// Not implemented since this is going away
	}
}

// TODO: cloning persistent map for the above

struct Bucket {
	k: ByteBox,
	v: ByteBox,
}

impl Bucket {
}

struct BucketPtr {
	v: Option<Bucket>,
}

impl BucketPtr {
	fn empty() -> BucketPtr {
		BucketPtr {
			v: None,
		}
	}

	fn set(&mut self, b: Bucket) {
		self.v = Some(b);
	}

	fn new<V: Datum>(k: &[u8], v: &V) -> BucketPtr {
		BucketPtr {
			v: Some(Bucket {
				k: ByteBox::from_key(k),
				v: ByteBox::from_value(v),
			}),
		}
	}
}

struct Node2Ptr {
	v: Option<Box<Node2>>,
}

impl Node2Ptr {
	fn new(p: Node2) -> Node2Ptr {
		Node2Ptr {
			v: Some(Box::new(p)),
		}
	}

	fn new_from_box(p: Box<Node2>) -> Node2Ptr {
		Node2Ptr {
			v: Some(p),
		}
	}

	fn empty() -> Node2Ptr {
		Node2Ptr {
			v: None,
		}
	}

	fn set(&mut self, n: Node2) {
		self.v = Some(Box::new(n));
	}

	/// Like flush, but this node is a root node. May result in the creation of a new parent.
	/// Flushing is the only operation allowed to create new nodes. In addition,
	/// this is the only operation allowed to create a new level of nodes, always at the top of the tree,
	/// so the tree is always fully balanced.
	fn flush_for_root(self) -> Self {
		match self.v {
			Some(mut bp) => match (*bp).flush() {
				Some((new_bucket, new_node)) => Self::new(Node2::new_from_two(Self::new_from_box(bp), new_bucket, new_node)),
				// need to use new here because self.v is a move (it needs to be so it's mutable)
				None => Self::new_from_box(bp),
			},
			None => Self::empty(),
		}
	}

	/// Like insert, but where this node is a root node.
	fn insert_for_root<D: Datum>(self, k: &[u8], v: &D) -> Self {
		let mut newself = self.flush_for_root();
		match newself.v {
			Some(ref mut bn) => (*bn).insert(k, v),
			None => newself = Self::new(Node2::new_from_one(k, v)),
		}
		newself
	}

	fn get_for_root(&mut self, k: &[u8]) -> Option<&ByteBox> {
		match self.v {
			Some(ref mut bn) => (*bn).get(k),
			None => None,
		}
	}

	fn check_invariants(&self) {
		match self.v {
			Some(ref bn) => (*bn).check_invariants(),
			None => (),
		}
	}
}

// TODO: figure out a good r/w interface for packing/unpacking nodes.
// Packer/Unpacker<T>?
struct Node2 {
	// Invariant: Height is max(height(children)) + 1 and no more than min(height(children)) + 2.
	// If a leaf, height == 0.
	// height: u8,
	/// Invariant: between (NODE_CAPACITY - 1) / 2 and NODE_CAPACITY - 1 unless we are the top node,
	/// in which case this is between 0 and NODE_CAPACITY - 1.
	/// When flushed, this is between 0 and NODE_CAPACITY - 2.
	count_buckets: u16,
	// count_bytes: u16,
	/// Buckets: key z pairs in this node.
	/// Invariant: the buckets in the interval [0, count_children - 1) are populated,
	/// all others are not.
    buckets: [BucketPtr; NODE_CAPACITY as usize - 1],
	/// Invariant: if not a leaf, the buckets in the interval [0, count_children) are populated,
	/// all others are not.
	/// If this is a leaf, all children are empty.
    children: [Node2Ptr; NODE_CAPACITY as usize],
}

impl Node2 {
	fn empty() -> Node2 {
		unsafe {
			Node2 {
				// height: 0,
				count_buckets: 0,
				// We could use mem::uninitialized, but this is a test class.
        	    buckets: make_array!(|_| BucketPtr::empty(), (NODE_CAPACITY - 1) as usize),
       	    	children: make_array!(|_| Node2Ptr::empty(), NODE_CAPACITY as usize),
			}
		}
	}

	fn new_from_one<V: Datum>(k: &[u8], v: &V) -> Node2 {
		let mut r = Self::empty();
		r.buckets[0] = BucketPtr::new(k, v);
		r.count_buckets = 1;
		r
	}

	fn new_from_two(n1: Node2Ptr, b1: BucketPtr, n2: Node2Ptr) -> Node2 {
		let mut r = Self::empty();

		r.buckets[0] = b1;
		r.children[0] = n1;
		r.children[1] = n2;
		r.count_buckets = 1;

		r
	}

	fn is_leaf(&self) -> bool {
		self.children[0].v.is_none()
	}

	fn needs_flush(&self) -> bool {
		self.count_buckets == NODE_CAPACITY - 1
	}

	fn get_bucket(&self, idx: usize) -> &Bucket {
		self.buckets[idx].v.as_ref().unwrap()
	}

	fn get_child(&self, idx: usize) -> &Node2 {
		self.children[idx].v.as_ref().unwrap()
	}

	fn get_bucket_mut(&mut self, idx: usize) -> &mut Bucket {
		self.buckets[idx].v.as_mut().unwrap()
	}

	fn get_child_mut(&mut self, idx: usize) -> &mut Node2 {
		self.children[idx].v.as_mut().unwrap()
	}

	/// Returns the first bucket greater than or equal to the given key, or None if this key is the greatest.
	/// Returns true if a direct match.
	/// Precondition: Not a leaf.
	fn find_bucket(&self, k: &[u8]) -> Result<usize, usize> {
		// TODO: we can make this faster with a subslice.
		self.buckets[0..(self.count_buckets as usize)].binary_search_by(|bp| bp.v.as_ref().unwrap().k.bytes().cmp(k.bytes()))
	}

	/// Helper fn for inserting into an array. We assume there is room in the array, and it is ok to overwrite
	/// the T at position arrsize.
	fn insert_into_slice<T>(arr: &mut [T], item: T, idx: usize, arrsize: usize) {
		for i in 0..(arrsize - idx) {
			arr.swap(arrsize - i, arrsize - i - 1);
		}
		arr[idx] = item;
	}

	/// Precondition: Is a leaf, fully flushed.
	fn insert_leaf<V: Datum>(&mut self, idx: usize, k: &[u8], v: &V) -> ()
	{
		debug_assert!(!self.needs_flush());
		Self::insert_into_slice(self.buckets.as_mut(), BucketPtr::new(k, v), idx, self.count_buckets as usize);
		// TODO: count_buckets -> bucket_count
		self.count_buckets += 1;
	}

	/// Requirements: b.key is between bucket[idx].key and bucket[idx + 1].key, if the latter exists,
	/// or merely greater than bucket[idx].key if not.
	/// The values descended from n are *greater* than b.key, and less than bucket[idx + 1].key.
	/// Precondition: Not a leaf.
	fn insert_sibling(&mut self, idx: usize, b: BucketPtr, n: Node2Ptr) {
		panic!("this is wrong");
		unsafe {
			// We don't do this because this is a test tree.
			// ptr::copy(self.buckets[idx], self.buckets[idx + 1], count_values - idx);
			// ptr::write(self.buckets[idx], b);
			// ptr::copy(self.children[idx], self.children[idx + 1], count_values - idx);
			// ptr::write(self.children[idx], n);

			// Move each child and bucket over, starting with the last one.
			for i in 0..((self.count_buckets as usize) + 1 - idx) {
				let from_idx = self.count_buckets as usize + 1 - i;
				let to_idx = from_idx + 1;

				self.children.swap(from_idx, to_idx);
				if i != 0 {
					// Recall there's count_buckets buckets and count_buckets + 1children.
					self.buckets.swap(from_idx, to_idx);
				}
			}

			self.buckets[idx] = b;
			self.children[idx] = n;

			self.count_buckets += 1;
		}
	}

	/// Fully flushes this node, making it ready for insertion. May cause the node to split. Does not modify children.
	/// Flushing is the only operation allowed to create new nodes. In addition, this particular flush
	/// may not change the level of any node of the tree, so a fully balanced tree remains so.
	/// Preconditions: None.
	/// Postconditions: This node is fully flushed.
	/// Returns: If this node was split, the bucket and node pointer that should be inserted into a parent node.
	/// Note that the bucket should be this node's new parent bucket, and the new node should inherit the old bucket.
	fn flush(&mut self) -> Option<(BucketPtr, Node2Ptr)> {
		if self.needs_flush() {
			// Split down the middle. If we have (2n + 1) buckets, this picks bucket n + 1, exactly in the middle.
			// If we have 2n buckets the nodes will be uneven so we pick n + 1, saving us one bucket copy.
			let count_buckets = self.count_buckets as usize;
			let split_idx = count_buckets / 2 + 1;
			let mut n2 = Self::empty();

			for i in (split_idx + 1)..count_buckets {
				let dst_idx = i - split_idx - 1; // start from 0 in dst
				mem::swap(&mut self.buckets[i], &mut n2.buckets[dst_idx]);
				// Note that this is safe even if we are a leaf. For now.
				mem::swap(&mut self.children[i], &mut n2.children[dst_idx]);
			}
			// Don't forget the last child
			// Note that this is safe even if we are a leaf. For now.
			mem::swap(&mut self.children[count_buckets], &mut n2.children[count_buckets - split_idx - 1]);

			// Now our children are divided among two nodes. This leaves an extra bucket, which we return
			// so the parent node can do something with it.
			let mut bp = BucketPtr::empty();
			let mut n2p = Node2Ptr::empty();
			mem::swap(&mut bp, &mut self.buckets[split_idx]);
			n2p.set(n2);

			Some((bp, n2p))
		} else {
			None
		}
	}

	/// Preconditions: This node is fully flushed.
	/// Postconditions: This node may need flushing at the next insert.
	fn insert<D: Datum>(&mut self, k: &[u8], v: &D) {
		debug_assert!(!self.needs_flush());
		match self.find_bucket(k) {
			Ok(idx) => {
				panic!("Duplicate key"); // TODO
			},
			Err(idx) => if self.is_leaf() {
				self.insert_leaf(idx, k, v)
			} else {
				// Insert in a child node.
				match self.get_child_mut(idx).flush() {
					Some((new_bucket_ptr, new_node_ptr)) => {
						// Need to insert a new bucket. May put us into a flushable state.
						self.insert_sibling(idx, new_bucket_ptr, new_node_ptr);
						match k.bytes().cmp(self.get_bucket_mut(idx).k.bytes()) {
							Ordering::Less => self.get_child_mut(idx).insert(k, v),
							Ordering::Equal => panic!("Duplicate key"), // TODO
							Ordering::Greater => self.get_child_mut(idx + 1).insert(k, v),
						}
					}
					None => {
						self.get_child_mut(idx).insert(k, v)
					}
				}
			},
		}
	}

	fn get(&self, k: &[u8]) -> Option<&ByteBox> {
		match self.find_bucket(k) {
			Ok(idx) => Some(&self.get_bucket(idx).v),
			Err(idx) => {
				if self.is_leaf() {
					None
				} else {
					self.get_child(idx).get(k)
				}
			},
		}
	}

	fn check_invariants_helper(&self, parent_lower_bound: Option<&ByteBox>, parent_upper_bound: Option<&ByteBox>) {
		// TODO: validate all leaves are at the same level

		// Validate the bucket count
		for i in 0..(NODE_CAPACITY as usize - 1) {
			if i >= self.count_buckets as usize {
				assert!(self.buckets[i].v.is_none());
			} else {
				assert!(self.buckets[i].v.is_some());
				// Validate sorted order
				if i > 1 {
					assert!(self.get_bucket(i).k < self.get_bucket(i - 1).k);
				}
			}
		}

		// Validate bounds
		assert!(parent_lower_bound.is_none() || &self.get_bucket(0).k > parent_lower_bound.unwrap());
		assert!(parent_upper_bound.is_none() || &self.get_bucket(self.count_buckets as usize - 1).k < parent_upper_bound.unwrap());

		// Validate the children
		for i in 0..(NODE_CAPACITY as usize) {
			if self.is_leaf() || i >= self.count_buckets as usize + 1 {
				assert!(self.children[i].v.is_none());
			} else {
				assert!(self.children[i].v.is_some());

				let lower_bound: Option<&ByteBox>;
				if i == 0 {
					lower_bound = None;
				} else {
					lower_bound = Some(&self.get_bucket(i - 1).k);
				}

				let upper_bound: Option<&ByteBox>;
				if i == self.count_buckets as usize {
					upper_bound = None;
				} else {
					upper_bound = Some(&self.get_bucket(i).k);
				}

				self.get_child(i).check_invariants_helper(lower_bound, upper_bound);
			}
		}
	}

	fn check_invariants(&self) {
		self.check_invariants_helper(None, None);
	}
}

// The data structure that will be used for our integrity tests.
pub struct PersistentBTree {
	head: Node2Ptr,
}

impl PersistentBTree {
	pub fn new() -> PersistentBTree {
		PersistentBTree {
			head: Node2Ptr::empty(),
		}
	}
}

impl ByteMap for PersistentBTree {
	type D = ByteBox;

	fn insert<K: Key + ?Sized, V: Datum>(&mut self, k: &K, v: &V) -> () {
		// Dummy to make the compiler behave. Since we're dealing in Options and Boxes, shouldn't have a runtime cost.
		let mut dummy = Node2Ptr::empty();
		mem::swap(&mut dummy, &mut self.head);
		self.head = dummy.insert_for_root(k.bytes(), v);
	}

	fn get<K: Key + ?Sized>(&mut self, k: &K) -> Option<&Self::D> {
		self.head.get_for_root(k.bytes())
	}

	fn delete<K: Key + ?Sized>(&mut self, k: &K) -> bool {
		panic!("Not implemented")
	}

	fn check_invariants(&self) {
		self.head.check_invariants();
	}
}
