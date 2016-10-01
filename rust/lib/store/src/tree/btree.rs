use std::borrow::Borrow;
use std::cmp::{Ord, Ordering};
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

	fn new<V: Datum>(k: &[u8], v: &V) -> BucketPtr {
		BucketPtr {
			v: Some(Bucket {
				k: ByteBox::from_key(k),
				v: ByteBox::from_value(v),
			}),
		}
	}

	fn unwrap(&self) -> &Bucket {
		self.v.as_ref().unwrap()
	}

	fn unwrap_mut(&mut self) -> &mut Bucket {
		self.v.as_mut().unwrap()
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

	fn unwrap(&self) -> &Node2 {
		self.v.as_ref().unwrap().as_ref()
	}

	fn unwrap_mut(&mut self) -> &mut Node2 {
		self.v.as_mut().unwrap().as_mut()
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
		// TODO probably can clean this up
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

	fn delete_for_root(self, k: &[u8]) -> (Self, bool) {
		// to get around borrow checker
		let newself = self;

		// TODO probably can clean this up
		match newself.v {
			Some(mut bn) => {
				let r = bn.as_mut().delete(k);
				if bn.as_ref().count_buckets == 0 {
					let mut n = *bn;
					if n.is_leaf() {
						// We're empty!
						(Self::empty(), r)
					} else {
						// There must be 0 buckets and 1 child
						let mut new_node = Node2Ptr::empty();
						mem::swap(&mut new_node, &mut n.children[0]);
						debug_assert!(new_node.v.is_some());
						(new_node, r)
					}
				} else {
					(Self::new_from_box(bn), r)
				}
			}
			None => (Self::empty(), false),
		}
	}

	fn check_invariants(&self) {
		match self.v.as_ref() {
			Some(bn) => (*bn).check_invariants(),
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

	/// A node is deficient iff it can be merged with another deficient node
	/// without needing a flush.
	fn is_deficient(&self) -> bool {
		self.count_buckets < (NODE_CAPACITY - 1) / 2
	}

	// TODO dont need these
	fn get_bucket(&self, idx: usize) -> &Bucket {
		self.buckets[idx].unwrap()
	}

	fn get_child(&self, idx: usize) -> &Node2 {
		self.children[idx].unwrap()
	}

	fn get_bucket_mut(&mut self, idx: usize) -> &mut Bucket {
		self.buckets[idx].unwrap_mut()
	}

	fn get_child_mut(&mut self, idx: usize) -> &mut Node2 {
		self.children[idx].unwrap_mut()
	}

	/// Returns Ok(i) if bucket[i]'s key is equal to the given key.
	/// Returns Err(i) where bucket[i] has the first key greater than the given key.
	/// If buckets are empty (which probably shouldn't happen), returns Err(0).
	fn find_bucket(&self, k: &[u8]) -> Result<usize, usize> {
		// TODO: we can make this faster with a subslice.
		self.buckets[0..(self.count_buckets as usize)].binary_search_by(
			|bp| bp.v.as_ref().unwrap().k.bytes().cmp(k.bytes()))
	}

	/*
	Safe array rotation functions. In the long run, we want to replace most usages with memmoves
	and uninitialized data.
	*/
	fn rotate_right<T>(arr: &mut [T], pos: usize) {
		// The 'swapping hands' algorithm. There are algorithms with faster constant time factors for large input,
		// but we don't have large input. TODO: investigate the above claim.

		// Suppose we start with abcde and want to end up with cdeab. pos = 2.
		// arr = abcde
		arr.reverse();
		// arr = edcba
		arr[..pos].reverse();
		// arr = decba
		arr[pos..].reverse();
		// arr = deabc
	}

	fn rotate_left<T>(arr: &mut [T], pos: usize) {
		// borrow checker...
		let len = arr.len() - pos;
		Self::rotate_right(arr, len);
	}

	fn swap<T>(a: &mut [T], b: &mut [T]) {
		if a.len() != b.len() {
			panic!("mismatched slice swap");
		}

		// (Probably) autovectorized. TODO: check, but this code might be going away anyway.
		for i in 0..a.len() {
			mem::swap(&mut a[i], &mut b[i]);
		}
	}

	/// Helper fn for inserting into an array. We assume there is room in the array, and it is ok to overwrite
	/// the T at position arrsize.
	// TODO: should be utility function.
	fn rotate_in<T>(item: &mut T, arr: &mut [T]) {
		// Unfortunately we must be a little unsafe here, even though this is supposed to be a foolproof fn
		// Optimizer should remove the extra mem copies
		let mut dummy: [T; 1] = unsafe { mem::uninitialized() };
		mem::swap(item, &mut dummy[0]);
		Self::rotate_in_slice(dummy.as_mut(), arr);
		mem::swap(item, &mut dummy[0]);
		mem::forget(dummy); // How odd this isn't unsafe...
	}

	fn rotate_out<T>(arr: &mut [T], item: &mut T) {
		// Unfortunately we must be a little unsafe here, even though this is supposed to be a foolproof fn
		// Optimizer should remove the extra mem copies
		let mut dummy: [T; 1] = unsafe { mem::uninitialized() };
		mem::swap(item, &mut dummy[0]);
		Self::rotate_out_slice(arr, dummy.as_mut());
		mem::swap(item, &mut dummy[0]);
		mem::forget(dummy); // How odd this isn't unsafe...
	}

	fn rotate_in_slice<T>(src: &mut [T], dst: &mut [T]) {
		// Borrow checker tricks
		let srclen = src.len();

		Self::rotate_right(dst, srclen);
		Self::swap(src, &mut dst[..srclen]);
	}

	fn rotate_out_slice<T>(src: &mut [T], dst: &mut [T]) {
		// Borrow checker tricks
		let dstlen = dst.len();
		// Rotating forward len - n is equivalent to rotating backward n
		// let len = src.len() - dstlen;

		Self::swap(&mut src[..dstlen], dst);
		Self::rotate_left(src, dstlen);
	}

	/// Precondition: Is a leaf, fully flushed.
	fn insert_leaf<V: Datum>(&mut self, idx: usize, k: &[u8], v: &V) -> ()
	{
		debug_assert!(!self.needs_flush());
		debug_assert!(self.is_leaf());

		Self::rotate_in(&mut BucketPtr::new(k, v), &mut self.buckets[idx..(self.count_buckets as usize + 1)]);
		// TODO: count_buckets -> bucket_count
		self.count_buckets += 1;
	}

	/// Requirements: b.key is between bucket[idx].key and bucket[idx + 1].key, if the latter exists,
	/// or merely greater than bucket[idx].key if not.
	/// The values descended from n are *greater* than b.key, and less than bucket[idx + 1].key.
	/// Precondition: Not a leaf, fully flushed.
	fn insert_sibling(&mut self, idx: usize, mut b: BucketPtr, mut n: Node2Ptr) {
		debug_assert!(!self.needs_flush());
		debug_assert!(!self.is_leaf());

		Self::rotate_in(&mut b, &mut self.buckets[idx..(self.count_buckets as usize + 1)]);
		Self::rotate_in(&mut n, &mut self.children[(idx + 1)..(self.count_buckets  as usize + 2)]);
		self.count_buckets += 1;
	}

	/// Fully flushes this node, making it ready for insertion. May cause the node to split. Does not modify children.
	/// Flushing is the only operation allowed to create new nodes. In addition, this particular flush
	/// may not change the level of any node of the tree, so a fully balanced tree remains so.
	/// Preconditions: None.
	/// Result: This node is fully flushed.
	/// Returns: If this node was split, the bucket and node pointer that should be inserted into a parent node.
	/// Note that the bucket should be this node's new parent bucket, and the new node should inherit the old bucket.
	fn flush(&mut self) -> Option<(BucketPtr, Node2Ptr)> {
		if self.needs_flush() {
			let count_buckets = self.count_buckets as usize;
			// Split down the middle. If we have (2n + 1) buckets, this picks bucket n (0-indexed), exactly in the middle.
			// If we have 2n buckets the nodes will be uneven so we pick n, saving us one bucket copy.
			let split_idx = count_buckets / 2;
			let mut n2 = Self::empty();

			// TODO: use rotate fns
			for i in (split_idx + 1)..count_buckets {
				let dst_idx = i - split_idx - 1; // start from 0 in dst
				mem::swap(&mut self.buckets[i], &mut n2.buckets[dst_idx]);
				// Note that this is safe even if we are a leaf, because if so, all children are empty.
				mem::swap(&mut self.children[i], &mut n2.children[dst_idx]);
			}
			// Don't forget the last child
			mem::swap(&mut self.children[count_buckets], &mut n2.children[count_buckets - split_idx - 1]);

			n2.count_buckets = self.count_buckets - split_idx  as u16 - 1;
			self.count_buckets = split_idx as u16;

			// Now our children are divided among two nodes. This leaves an extra bucket, which we return
			// so the parent node can do something with it.
			let mut bp = BucketPtr::empty();
			mem::swap(&mut bp, &mut self.buckets[split_idx]);

			// We are done, time to return.
			// println!("split {} {} -> {} {}", count_buckets, split_idx, self.count_buckets, n2.count_buckets);

			// self.quickcheck_invariants();
			// n2.quickcheck_invariants();

			let mut n2p = Node2Ptr::empty();
			n2p.set(n2);

			Some((bp, n2p))
		} else {
			None
		}
	}

	/// Preconditions: The parent node, if it exists, is not deficient. Both args are deficient.
	fn merge_deficient_nodes(&mut self, right_other: &mut Self, bucket_to_steal: &mut BucketPtr) {
		debug_assert!(self.is_deficient() && right_other.is_deficient());
		let count_buckets = self.count_buckets as usize;
		let other_count_buckets = right_other.count_buckets as usize;

		// We are 'stealing' this bucket from the parent
		mem::swap(&mut self.buckets[count_buckets], bucket_to_steal);
		Self::swap(
			&mut self.buckets[(count_buckets + 1)..(count_buckets + other_count_buckets + 1)],
			&mut right_other.buckets[..other_count_buckets]);
		Self::swap(
			&mut self.children[(count_buckets + 1)..(count_buckets + other_count_buckets + 2)],
			&mut right_other.children[..(other_count_buckets + 1)]);
		self.count_buckets += right_other.count_buckets + 1;

		// self.check_invariants();
	}

	fn borrow_left(&mut self, left: &mut Node2Ptr, left_parent_bucket: &mut BucketPtr) {
		let mut ln = left.unwrap_mut();
		let num_to_borrow = (ln.count_buckets - self.count_buckets + 1) / 2;
		debug_assert!(num_to_borrow >= 1, "invalid bucket sizes: {} {}", ln.count_buckets, self.count_buckets);
		let count_buckets = self.count_buckets as usize;

		// Make room for the things we're borrowing.
		Self::rotate_right(&mut self.buckets[..(count_buckets + num_to_borrow as usize)], num_to_borrow as usize);
		Self::rotate_right(&mut self.children[..(count_buckets + num_to_borrow as usize + 1)], num_to_borrow as usize);

		// The first bororwed bucket becomes the new parent bucket, and the old parent bucket is moved into this node.
		mem::swap(left_parent_bucket, &mut self.buckets[num_to_borrow as usize - 1]);
		mem::swap(&mut ln.buckets[(ln.count_buckets - num_to_borrow) as usize], left_parent_bucket);

		// Swap in the buckets and children. Empty buckets will be swapped into ln.
		// We borrow n children and n-1 buckets (additionally, we did the little shuffle above, borrowing n total).
		Self::swap(
			&mut ln.buckets[(ln.count_buckets - num_to_borrow + 1) as usize..ln.count_buckets as usize],
			&mut self.buckets[..(num_to_borrow as usize - 1)]);
		Self::swap(
			&mut ln.children[(ln.count_buckets - num_to_borrow + 1) as usize..(ln.count_buckets as usize + 1)],
			&mut self.children[..(num_to_borrow as usize)]);

		self.count_buckets += num_to_borrow;
		ln.count_buckets -= num_to_borrow;

		// self.quickcheck_invariants();
		// ln.quickcheck_invariants();
			// self.check_invariants();
			// ln.check_invariants();
	}

	fn borrow_right(&mut self, right: &mut Node2Ptr, right_parent_bucket: &mut BucketPtr) {
		let mut rn = right.unwrap_mut();
		let num_to_borrow = (rn.count_buckets - self.count_buckets + 1) / 2;
		debug_assert!(num_to_borrow >= 1, "invalid bucket sizes: {} {}", self.count_buckets, rn.count_buckets);
		let count_buckets = self.count_buckets as usize;

		// The last bororwed bucket becomes the new parent bucket, and the old parent bucket is moved into this node.
		mem::swap(right_parent_bucket, &mut self.buckets[count_buckets]);
		mem::swap(&mut rn.buckets[num_to_borrow as usize - 1], right_parent_bucket);

		// Swap in the buckets and children. Empty buckets will be swapped into rn.
		// We borrow n children and n-1 buckets (additionally, we did the little shuffle above, borrowing n total).
		Self::swap(
			&mut rn.buckets[0..(num_to_borrow - 1) as usize],
			&mut self.buckets[(count_buckets + 1)..(count_buckets + num_to_borrow as usize)]);
		Self::swap(
			&mut rn.children[0..num_to_borrow as usize],
			&mut self.children[(count_buckets + 1)..(count_buckets + num_to_borrow as usize + 1)]);

		// Adjust the positions in the right node.
		Self::rotate_left(&mut rn.buckets[0..(rn.count_buckets ) as usize], num_to_borrow as usize);
		Self::rotate_left(&mut rn.children[0..(rn.count_buckets + 1) as usize], num_to_borrow as usize);

		self.count_buckets += num_to_borrow;
		rn.count_buckets -= num_to_borrow;

		// self.quickcheck_invariants();
		// rn.quickcheck_invariants();
			// self.check_invariants();
			// rn.check_invariants();
	}

	/// Preconditions: The parent node, if it exists, is not deficient.
	/// (It may become deficient as a result of this merge.) As such,
	/// at least one neighbor is guaranteed.
	/// After this is run, one bucket pointer may be empty. If so, the parent must delete
	/// that bucket.

	// TODO: ptrs and bucketptrs are already options...
	fn merge(&mut self, left: Option<&mut Node2Ptr>, left_parent_bucket: Option<&mut BucketPtr>,
		right: Option<&mut Node2Ptr>, right_parent_bucket: Option<&mut BucketPtr>)
	-> Option<bool> {
		debug_assert!(self.is_deficient());
		debug_assert!(left.is_some() || right.is_some());
		debug_assert!(left.is_some() == left_parent_bucket.is_some());

		match left {
			Some(lv) => {
				if lv.unwrap().is_deficient() {
					lv.unwrap_mut().merge_deficient_nodes(self, left_parent_bucket.unwrap());
					Some(false)
				} else {
					match right {
						Some(rv) => {
							if rv.unwrap().is_deficient() {
								self.merge_deficient_nodes(rv.unwrap_mut(), right_parent_bucket.unwrap());
								Some(true)
							} else if rv.unwrap().count_buckets > lv.unwrap().count_buckets {
								self.borrow_right(rv, right_parent_bucket.unwrap());
								None
							} else {
								self.borrow_left(lv, left_parent_bucket.unwrap());
								None
							}
						}
						None => {
							self.borrow_left(lv, left_parent_bucket.unwrap());
						    None
						}
					}
				}
			}
			None => {
				if right.as_ref().unwrap().unwrap().is_deficient() {
					self.merge_deficient_nodes(right.unwrap().unwrap_mut(), right_parent_bucket.unwrap());
					Some(true)
				} else {
					self.borrow_right(right.unwrap(), right_parent_bucket.unwrap());
					None
				}
			}
		}
	}

	/// Like split_at_mut but with reasonable defaults for out of bounds indices.
	fn split_or_empty<T>(t: &mut [T], idx: isize) -> (&mut [T], &mut [T]) {
		if idx >= 0 {
			if (idx as usize) < t.len() {
				t.split_at_mut(idx as usize)
			} else {
				(t, [].as_mut())
			}
		} else {
			([].as_mut(), t)
		}
	}

	/// Returns true if merging happened (and hence buckets[idx] has changed).
	// TODO: this code is pure shit.
	fn check_deficient_child(&mut self, idx: usize) {
		debug_assert!(idx <= self.count_buckets as usize);

		if self.get_child(idx).is_deficient() {
			let delete_result;
			let count_buckets = self.count_buckets as usize;

			{
				// Get the borrows we need, in a way that won't make Rust freak out.
				// (A borrow by index borrows the whole array.)

				// If the idx is n, left child and right child are at n - 1 and n + 1.
				// Left bucket and right bucket are at n - 1 and n.
				let (left_bucket_dummy, right_bucket_dummy) = Self::split_or_empty(&mut self.buckets, idx as isize);
				let (left_child_dummy, mut mid_and_right_child_dummy) = Self::split_or_empty(&mut self.children, idx as isize);
				let (mid_child_dummy, right_child_dummy) = Self::split_or_empty(&mut mid_and_right_child_dummy, 1);

				// Now we have the borrows we need, we can start bundling our arguments.
				// TODO: ptrs are already options...
				let left_sibling;
				let left_parent_bucket;
				let right_sibling;
				let right_parent_bucket;
				let child = mid_child_dummy[0].v.as_mut().unwrap();

				if idx > 0 {
					left_sibling = Some(&mut left_child_dummy[idx - 1]);
					left_parent_bucket = Some(&mut left_bucket_dummy[idx - 1]);
				} else {
					left_sibling = None;
					left_parent_bucket = None;
				}

				if idx < count_buckets {
					right_sibling = Some(&mut right_child_dummy[0]);
					right_parent_bucket = Some(&mut right_bucket_dummy[0]);
				} else {
					right_sibling = None;
					right_parent_bucket = None;
				}

				// A lot of syntax nonsense for one call!
				delete_result = child.merge(left_sibling, left_parent_bucket, right_sibling, right_parent_bucket);
			}

			match delete_result {
				// false == left bucket, middle child was deleted
				Some(false) => {
					Self::rotate_out(&mut self.buckets[(idx - 1)..count_buckets], &mut BucketPtr::empty());
					Self::rotate_out(&mut self.children[idx..(count_buckets + 1)], &mut Node2Ptr::empty());
					self.count_buckets -= 1;
				}
				// true = right bucket, right sibling was deleted
				Some(true) => {
					Self::rotate_out(&mut self.buckets[idx..count_buckets], &mut BucketPtr::empty());
					Self::rotate_out(&mut self.children[(idx + 1)..(count_buckets + 1)], &mut Node2Ptr::empty());
					self.count_buckets -= 1;
				}
				None => (),
			}

			// self.quickcheck_invariants();
			// self.check_invariants();
		}
	}

	// For the special case where head just has one bucket.
	fn collapse_maybe(mut self) -> Self {
		self.check_deficient_child(0);
		// Make sure we have a bucket left!
		if self.count_buckets == 0 {
			let mut dummy = Node2Ptr::empty();
			mem::swap(&mut self.children[0], &mut dummy);
			*dummy.v.unwrap()
		} else {
			self
		}
	}

	/// Preconditions: This node is fully flushed.
	/// Postconditions: This node may need flushing at the next insert.
	fn insert<D: Datum>(&mut self, k: &[u8], v: &D) {
		debug_assert!(!self.needs_flush());
		match self.find_bucket(k) {
			Ok(_) => {
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
							Ordering::Greater => self.get_child_mut(idx + 1).insert(k, v),
							Ordering::Equal => panic!("Duplicate key"), // TODO
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

	/// Postcondition: May leave this node deficient. Will not leave descendant nodes deficient.
	fn yank_rightmost_bucket(&mut self) -> BucketPtr {
		let count_buckets = self.count_buckets as usize;

		if self.is_leaf() {
			let mut r = BucketPtr::empty();
			mem::swap(&mut r, &mut self.buckets[count_buckets - 1]);
			self.count_buckets -= 1;
			r
		} else {
			let r = self.get_child_mut(count_buckets).yank_rightmost_bucket();
			self.check_deficient_child(count_buckets);
			r
		}
	}

	/// Postcondition: May leave this node deficient. Will not leave descendant nodes deficient.
	fn yank_leftmost_bucket(&mut self) -> BucketPtr {
		if self.is_leaf() {
			let mut r = BucketPtr::empty();
			Self::rotate_out(&mut self.buckets[0..self.count_buckets as usize], &mut r);
			self.count_buckets -= 1;
			r
		} else {
			let r = self.get_child_mut(0).yank_leftmost_bucket();
			self.check_deficient_child(0);
			r
		}
	}

	/// Postcondition: May leave this node deficient.
	fn delete(&mut self, k: &[u8]) -> bool {
		// Unlike in insert, we rebalance *after* delete.
		match self.find_bucket(k) {
			Ok(idx) => {
				if self.is_leaf() {
					Self::rotate_out(&mut self.buckets[idx..self.count_buckets as usize], &mut BucketPtr::empty());
					self.count_buckets -= 1;
					// self.check_invariants();
					true
			    } else {
					if idx > 0 {
						// get leftmost descendant from right child
				 	    self.buckets[idx] = self.get_child_mut(idx + 1).yank_leftmost_bucket();
				 	    self.check_deficient_child(idx + 1);
				 	    true
					} else {
						// get rightmost descendant from left child
						self.buckets[idx] = self.get_child_mut(idx).yank_rightmost_bucket();
						self.check_deficient_child(idx);
				 	    true
				    }
			    }
			},
			Err(idx) => if !self.is_leaf() {
				let r = self.get_child_mut(idx).delete(k);
				self.check_deficient_child(idx);
				r
			} else {
				false
			},
		}
	}

	fn quickcheck_invariants(&self) {
		self.check_invariants_helper(None, None, false);
	}

	fn check_invariants_helper(&self, parent_lower_bound: Option<&ByteBox>, parent_upper_bound: Option<&ByteBox>, recurse: bool) {
		// TODO: validate all leaves are at the same level
		// TODO: validate that there are no sibling deficient nodes

		// Validate the bucket count
		for i in 0..(NODE_CAPACITY as usize - 1) {
			if i >= self.count_buckets as usize {
				assert!(self.buckets[i].v.is_none(), "expected empty bucket in position {}", i);
			} else {
				assert!(self.buckets[i].v.is_some(), "expected populated bucket in position {}", i);
				// Validate sorted order
				if i > 1 {
					assert!(self.get_bucket(i).k > self.get_bucket(i - 1).k);
				}
			}
		}

		// Validate bounds
		assert!(parent_lower_bound.is_none() || &self.get_bucket(0).k > parent_lower_bound.unwrap());
		assert!(parent_upper_bound.is_none() || &self.get_bucket(self.count_buckets as usize - 1).k < parent_upper_bound.unwrap());

		// Node bounds. TODO: for parents only
		// TODO: assert at least one bucket
		// assert!(self.is_leaf() || self.count_buckets >= (NODE_CAPACITY - 3) / 2);

		// Validate the children
		for i in 0..(NODE_CAPACITY as usize) {
			if self.is_leaf() || i >= self.count_buckets as usize + 1 {
				assert!(self.children[i].v.is_none());
			} else {
				assert!(self.children[i].v.is_some());

				if recurse {
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

					self.get_child(i).check_invariants_helper(lower_bound, upper_bound, recurse);
				}
			}
		}
	}

	fn check_invariants(&self) {
		self.check_invariants_helper(None, None, true);
	}
}

// A simple in-memory persistent b-tree.
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
		let mut dummy = Node2Ptr::empty();
		mem::swap(&mut dummy, &mut self.head);
		let (newhead, r) = dummy.delete_for_root(k.bytes());
		self.head = newhead;
		r
	}

	fn check_invariants(&self) {
		self.head.check_invariants();
	}
}
