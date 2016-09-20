use std::borrow::Borrow;
use std::convert::TryFrom;
use std::mem;
use std::ptr;

use data::*;

// A brain-dead b-tree for testing/comparison.

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

// TODO move to common lib
#[derive(Debug, PartialEq, Eq)]
pub struct Value {
	// We box because Value (actually ValuePtr) must be sized.
	// Note that we use a Box inside the value, not on the outside. Why? Not sure, can't remember...
	v: Box<[u8]>,
}

impl Value {
	fn safe_new<D: Datum>(src: &D) -> Option<Value> {
		match u16::try_from(src.len()) {
			Ok(_) => Some(Value {
				v: src.box_copy(),
			}),
			Err(_) => None
		}
	}

	fn new<D: Datum>(src: &D) -> Value {
		Self::safe_new(src).unwrap()
	}

	pub fn unwrap(&self) -> &[u8] {
		&*self.v
	}
}

impl Datum for Value {
    fn len(&self) -> u16 {
    	u16::try_from(self.v.len()).unwrap() // should be safe
    }

    fn write_bytes<W: DataWrite>(&self, w: W) -> W::Result {
    	w.write(&*self.v)
    }
}

struct ValuePtr {
	v: Option<Value>,
}

impl ValuePtr {
	fn empty() -> ValuePtr {
		ValuePtr {
			v: None
		}
	}

	fn set(&mut self, v: Value) {
		self.v = Some(v);
	}
}

struct Node {
    vals: [ValuePtr; 16],
    // We actually only use this every other layer... but this is an intentionally lazy implementation.
    children: [NodePtr; 16],
}

impl Node {
	fn empty() -> Node {
		unsafe {
			Node {
        	    vals: make_array!(|_| ValuePtr::empty(), 16),
       	    	children: make_array!(|_| NodePtr::empty(), 16),
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
		self.vals[nibble as usize].set(Value::new(v));
	}

	// Interface functions
	fn get<B, I>(&mut self, k: I) -> Option<&Value> where
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

pub trait Tree {
	/// Note that we don't use IntoDatum values. Similarly, we don't have an IntoKey trait.
	/// The reason is we want conversion to be explicit.
	fn insert<V, BK, BV>(&mut self, k: BK, bv: BV) -> () where
	V: Datum,
	BK: Borrow<[u8]>,
	BV: Borrow<V>;

	/// This is mutable because gets may introduce read conflicts, and hence mutate the underlying datastructure.
	fn get<BK>(&mut self, k: BK) -> Option<&Value> where
	BK: Borrow<[u8]>;

	fn delete<BK>(&mut self, k: BK) -> bool where
	BK: Borrow<[u8]>;
}

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

impl Tree for BTree {
	fn insert<V, BK, BV>(&mut self, k: BK, bv: BV) -> () where
	V: Datum,
	BK: Borrow<[u8]>,
	BV: Borrow<V>,
	{
		self.head.insert(k.borrow().iter(), bv.borrow());
	}

	fn get<BK>(&mut self, k: BK) -> Option<&Value> where
	BK: Borrow<[u8]>,
	{
		self.head.get(k.borrow().iter())
	}

	fn delete<BK>(&mut self, k: BK) -> bool where
	BK: Borrow<[u8]>,
	{
		self.head.delete(k.borrow().iter())
	}
}
