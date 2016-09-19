#![cfg_attr(test, feature(plugin))]
#![cfg_attr(test, plugin(quickcheck_macros))]

use std::convert::TryFrom;
use std::marker::PhantomData;
use std::mem;
use std::ptr;

use data::*;
use data::slice::SliceDatumIterator;

#[cfg(test)]
#[macro_use]
extern crate quickcheck;

#[cfg(test)]
mod tests;

// A brain-dead b-tree for testing/comparison.

struct NodePtr<'a> {
	v: Option<Box<Node<'a>>>,
}

impl<'a> NodePtr<'a> {
	fn empty<'_>() -> NodePtr<'_> {
		NodePtr {
			v: None,
		}
	}

	fn set(&mut self, n: Node) {
		self.v = Some(Box::new(n));
	}
}

// TODO move to common lib
struct Value<'a> {
	// Note that we use a Box inside the value, not on the outside. Why? Not sure, can't remember...
	v: Box<[u8]>,
	p: PhantomData<&'a ()>,
}

impl<'a> Value<'a> {
	fn safe_new<D: Datum>(src: &D) -> Option<Value> {
		match u16::try_from(src.len()) {
			Ok(v) => Some(Value {
				v: src.box_copy(),
				p: PhantomData,
			}),
			Err(_) => None
		}
	}

	fn new<D: Datum>(src: &D) -> Value {
		Self::safe_new(src).unwrap()
	}
}

impl<'a> Datum for Value<'a> {
    fn len(&self) -> u16 {
    	u16::try_from(self.v.len()).unwrap() // should be safe
    }

    fn write_bytes<W: DataWrite>(&self, w: W) -> W::Result {
    	w.write(&*self.v)
    }

    type Stream = &'a Self;
    fn as_stream(&self) -> &'a Value<'a> {
    	self
    }
}

impl<'a> IntoIterator for &'a Value<'a> {
    type Item = u8;
    type IntoIter = SliceDatumIterator<'a>;

    fn into_iter(self) -> SliceDatumIterator<'a> {
        SliceDatumIterator::new(&*self.data)
    }
}

struct ValuePtr<'a> {
	v: Option<Value<'a>>,
}

impl<'a> ValuePtr<'a> {
	fn empty<'_>() -> ValuePtr<'_> {
		ValuePtr {
			v: None
		}
	}

	fn set(&mut self, v: Value) {
		self.v = Some(v);
	}
}

struct Node<'a> {
    vals: [ValuePtr<'a>; 16],
    // We actually only use this every other layer... but this is an intentionally lazy implementation.
    children: [NodePtr<'a>; 16],
}

impl<'a> Node<'a> {
	fn empty<'_>() -> Node<'_> {
		unsafe {
			Node {
        	    vals: make_array!(|_| ValuePtr::empty(), 16),
       	    	children: make_array!(|_| NodePtr::empty(), 16),
			}
		}
	}

	fn get_or_create_child(&mut self, nibble: u8) -> &mut Node {
		// TODO safety check in debug mode
		let mut ptr = &mut self.children[nibble as usize];
		match ptr.v {
			Some(ref mut b) => b,
			None => {
				ptr.set(Self::empty());
				ptr.v.as_mut().unwrap() // now safe
			}
		}
	}

	fn insert_for_hi_nibble<D: Datum, I: Iterator<Item = u8>>(&mut self, b: u8, k: &mut I, v: &D) {
		let n1 = (b & 0xf0) >> 4;
		let n2 = b & 0x0f;
		let mut child = self.get_or_create_child(n1);
		child.insert_for_lo_nibble(n2, k, v);
	}

	fn insert_for_lo_nibble<D: Datum, I: Iterator<Item = u8>>(&mut self, nibble: u8, k: &mut I, v: &D) {
		match k.next() {
			Some(b) => {
				let mut child = self.get_or_create_child(nibble);
				child.insert_for_hi_nibble(b, k, v);
			},
			None => self.finish_insert(nibble, v),
		}
	}

	fn finish_insert<D: Datum>(&mut self, nibble: u8, v: &D) {
		// TODO handle errors
		self.vals[nibble as usize].set(Value::new(v));
	}

	fn insert<D: Datum, I: Iterator<Item = u8>>(&mut self, k: &mut I, v: &D) {
		match k.next() {
			Some(b) => self.insert_for_hi_nibble(b, k, v),
			None => panic!("Tried to insert with empty key"), // TODO handle
		}
	}
}

struct Tree<'a> {
	head: Node<'a>,
}

impl<'a> Tree<'a> {
	fn new<'_>() -> Tree<'_> {
		Tree {
			head: Node::empty(),
		}
	}

	fn insert<DK: Datum, DV: Datum>(&mut self, k: &DK, v: &DV) {
		self.head.insert(k.iter(), v);
	}
}
