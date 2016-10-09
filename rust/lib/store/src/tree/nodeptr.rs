use tree::btree::*;

use data::*;
use data::slice::ByteRc;

pub struct NodePtr {
	v: Option<NodeRef>,
}

// TODO this class is redundant
impl NodePtr {
	pub fn new(p: HotNode) -> NodePtr {
		Self::wrap(NodeRef::new(p))
	}

	pub fn wrap(p: NodeRef) -> NodePtr {
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

	pub fn deref(&self) -> Option<&NodeRef> {
		self.v.as_ref()
	}

	pub fn unwrap(&self) -> &NodeRef {
		self.v.as_ref().unwrap()
	}

	pub fn deref_mut(&mut self) -> Option<&mut NodeRef> {
		self.v.as_mut()
	}

	pub fn unwrap_mut(&mut self) -> &mut NodeRef {
		self.v.as_mut().unwrap()
	}

	// pub fn cool(&mut self) {
	// 	if let Some(noderef) = self.v.as_mut() {
	// 		noderef.cool()
	// 	}
	// }

	// pub fn fork(&self) -> NodePtr {
	// 	match self.v.as_ref() {
	// 		Some(noderef) => Self::wrap(noderef.fork()),
	// 		None => Self::empty()
	// 	}
	// }

	pub fn insert<D: Datum>(self, k: &[u8], v: &D) -> Self {
		match self.v {
			Some(noderef) => Self::wrap(noderef.handle().insert(k, v)),
			None => Self::new(HotNode::new_from_one(k, v)),
		}
	}

	pub fn get(&mut self, k: &[u8]) -> Option<ByteRc> {
		match self.v.as_ref() {
			Some(bn) => (*bn).handle().get(k),
			None => None,
		}
	}

	// pub fn delete_for_root(self, k: &[u8]) -> (Self, bool) {
	// 	// to get around borrow checker
	// 	// let newself = self;

	// 	// TODO probably can clean this up
	// 	match newself.v.as_ref() {
	// 		Some(mut n) => {
	// 			let hn = n.heat();
	// 			let r = hn.delete(k);
	// 			if n.as_ref().bucket_count() == 0 {
	// 				let mut n = *bn;
	// 				if n.is_leaf() {
	// 					// We're empty!
	// 					(Self::empty(), r)
	// 				} else {
	// 					// There must be 0 buckets and 1 child
	// 					(n.disown_only_child(), r)
	// 				}
	// 			} else {
	// 				(Self::new(r), r)
	// 			}
	// 		}
	// 		None => (Self::empty(), false),
	// 	}
	// }

	pub fn check_invariants(&self) {
		match self.v.as_ref() {
			Some(bn) => (*bn).check_invariants(),
			None => (),
		}
	}
}
