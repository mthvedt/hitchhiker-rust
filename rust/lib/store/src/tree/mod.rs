use std::convert::TryFrom;
use std::mem;
use std::ptr;

use data::*;

#[cfg(test)]
mod tests;

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

	// fn new(n: Box<Node>) -> NodePtr {
	// 	NodePtr {
	// 		v: Some(n),
	// 	}
	// }

	fn set(&mut self, n: Node) {
		self.v = Some(Box::new(n));
	}
}

// TODO move to common lib
struct Value {
	// Note that we use a Box inside the value, not on the outside. Why? Not sure, can't remember...
	v: Box<[u8]>,
}

impl Value {
	fn safe_new<D: Datum>(src: &D) -> Option<Value> {
		match u16::try_from(src.len()) {
			Ok(v) => Some(Value { v: src.box_copy() }),
			Err(_) => None
		}
	}

	fn new<D: Datum>(src: &D) -> Value {
		Self::safe_new(src).unwrap()
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

	// fn new(n: Value) -> ValuePtr {
	// 	ValuePtr {
	// 		v: Some(n)
	// 	}
	// }

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
