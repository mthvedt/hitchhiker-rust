use std::mem;
use std::rc::Rc;
use std::cell::RefCell;
use std::ops::Deref;

use data::{ByteRc, Datum};

use tree::btree::*;
use tree::hotnode::*;
use tree::util::*;

// TODO: this doesn't deserve its own module. Move to HotNode?

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
			Some(bn) => (*bn).handle().apply(|n| n.check_invariants()),
			None => (),
		}
	}
}

/// A container for various kinds of nodes.
pub enum NodeRef {
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
	pub fn reassign(&mut self, nref: HotHandle) {
		match nref {
			HotHandle::Existing(_) => (), // debug_assert!(ptr_eq(self.acquire().lookup(), hn_ref.deref())),
			HotHandle::New(nref, hn_box_cell) => {
				debug_assert!(ptr_eq(self, nref.unwrap()));
				// TODO: can we do self = ?
				let mut replacement = NodeRef::Hot(hn_box_cell);
				mem::swap(&mut replacement, self)
			}
		}
	}

	/* Thermodynamics */

	/// Returns a hot NodeRef which may be modified, together with a reference to that node. May return self.
	pub fn heat<'a>(&'a self) -> (HotHandle<'a>, bool) {
		match self {
			&NodeRef::Hot(ref box_cell_hn) => (HotHandle::Existing(box_cell_hn.as_ref().borrow_mut()), false),
			&NodeRef::Warm(ref rc_hn) => (HotHandle::New(Some(self), Rc::new(RefCell::new((*rc_hn).fork()))), true),
		}
	}

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

	// /// Postcondition: May leave this node deficient.
	// pub fn delete(&mut self, k: &[u8]) -> bool {
	// 	// Unlike in insert, we rebalance *after* delete.
	// 	match self.find(k) {
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
}
