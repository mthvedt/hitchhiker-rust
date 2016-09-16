use std::convert::TryFrom;

use data::*;

// A brain-dead b-tree for testing/comparison.

struct Node {
    vals: [ValuePtr; 16],
    children: [NodePtr; 16],
}

struct NodePtr {
	v: Option<Box<Node>>,
}

impl NodePtr {
	fn empty() -> NodePtr {
		NodePtr {
			v: None
		}
	}

	fn new(n: Box<Node>) -> NodePtr {
		NodePtr {
			v: Some(n)
		}
	}
}

// TODO move to common lib
struct Value {
	// Note that we use a Box inside the value, not on the outside.
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

	fn new(n: Value) -> ValuePtr {
		ValuePtr {
			v: Some(n)
		}
	}
}
