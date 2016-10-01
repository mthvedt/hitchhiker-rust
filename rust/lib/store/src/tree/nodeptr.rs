use tree::btree::*;

use data::*;
use data::slice::ByteBox;

pub struct NodePtr {
	v: Option<Box<Node>>,
}

impl NodePtr {
	pub fn new(p: Node) -> NodePtr {
		NodePtr {
			v: Some(Box::new(p)),
		}
	}

	pub fn new_from_box(p: Box<Node>) -> NodePtr {
		NodePtr {
			v: Some(p),
		}
	}

	pub fn empty() -> NodePtr {
		NodePtr {
			v: None,
		}
	}

	pub fn is_empty(&self) -> bool {
		self.v.is_none()
	}

	pub fn set(&mut self, n: Node) {
		self.v = Some(Box::new(n));
	}

	pub fn deref(&self) -> Option<&Node> {
		self.v.as_ref().map(|rbn| rbn.as_ref())
	}

	pub fn unwrap(&self) -> &Node {
		self.v.as_ref().unwrap().as_ref()
	}

	pub fn deref_hot(&mut self) -> Option<&mut Node> {
		self.v.as_mut().map(|rbn| rbn.as_mut())
	}

	pub fn unwrap_hot(&mut self) -> &mut Node {
		self.v.as_mut().unwrap().as_mut()
	}

	/// Like flush, but this node is a root node. May result in the creation of a new parent.
	/// Flushing is the only operation allowed to create new nodes. In addition,
	/// this is the only operation allowed to create a new level of nodes, always at the top of the tree,
	/// so the tree is always fully balanced.
	pub fn flush_for_root(self) -> Self {
		match self.v {
			Some(mut bp) => match (*bp).flush() {
				Some((new_bucket, new_node)) => Self::new(Node::new_from_two(Self::new_from_box(bp), new_bucket, new_node)),
				// need to use new here because self.v is a move (it needs to be so it's mutable)
				None => Self::new_from_box(bp),
			},
			None => Self::empty(),
		}
	}

	/// Like insert, but where this node is a root node.
	pub fn insert_for_root<D: Datum>(self, k: &[u8], v: &D) -> Self {
		let mut newself = self.flush_for_root();
		match newself.v {
			Some(ref mut bn) => (*bn).insert(k, v),
			None => newself = Self::new(Node::new_from_one(k, v)),
		}
		newself
	}

	pub fn get_for_root(&mut self, k: &[u8]) -> Option<&ByteBox> {
		match self.v {
			Some(ref mut bn) => (*bn).get(k),
			None => None,
		}
	}

	pub fn delete_for_root(self, k: &[u8]) -> (Self, bool) {
		// to get around borrow checker
		let newself = self;

		// TODO probably can clean this up
		match newself.v {
			Some(mut bn) => {
				let r = bn.as_mut().delete(k);
				if bn.as_ref().bucket_count() == 0 {
					let mut n = *bn;
					if n.is_leaf() {
						// We're empty!
						(Self::empty(), r)
					} else {
						// There must be 0 buckets and 1 child
						(n.disown_only_child(), r)
					}
				} else {
					(Self::new_from_box(bn), r)
				}
			}
			None => (Self::empty(), false),
		}
	}

	pub fn check_invariants(&self) {
		match self.v.as_ref() {
			Some(bn) => (*bn).check_invariants(),
			None => (),
		}
	}
}
