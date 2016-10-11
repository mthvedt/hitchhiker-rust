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
	pub fn new(p: HotNode) -> Self {
		Self::wrap(NodeRef::new(p))
	}

	pub fn wrap(p: NodeRef) -> Self {
		NodePtr {
			v: Some(p),
		}
	}

	pub fn empty() -> Self {
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

	// Immtues this NodePtr, recurisvely immuting its children.
	pub fn cool(&mut self) {
		match self.v.as_mut() {
			Some(noderef) => noderef.cool(),
			None => (),
		}
	}

	pub fn fork(&self) -> Self {
		match self.v.as_ref() {
			Some(noderef) => Self::wrap(noderef.fork()),
			None => Self::empty(),
		}
	}

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
/// TODO: hot things should only be owned by one job at a time.
pub enum NodeRef {
	// TODO: either these two arms should be identical, or this shouldn't be RC.
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

	/// Debug helper for assign/reassign.
	fn hn_ptr(&self) -> *const HotNode {
		match self {
			&NodeRef::Hot(ref rc_rfc_hn) =>  rc_rfc_hn.deref().as_ptr(),
			&NodeRef::Warm(ref rc_hn) => rc_hn.deref() as *const _,
		}
	}

	/// Reassigns this node ref to the HotNode referred to by the given HotHandle.
	/// The given HotHandle must be a modified copy of this NodeRef (verified with debug assertions).
	pub fn reassign(&mut self, h: HotHandle) {
		match h {
			HotHandle::Existing(hn_ref) => {
				// Safety check: A HotHandle::Existing may only be reassigned to itself.
				if let &mut NodeRef::Hot(ref rc_rfc_hn) = self {
					debug_assert!(ptr_eq(rc_rfc_hn.deref().as_ptr(), hn_ref.deref() as *const _))
				} else {
					debug_assert!(false, "Cannot assign an existing HotNode to a warm NodeRef")
				}
			}
			HotHandle::New(h_nref, hn_rc_cell) => {
				let p = h_nref.unwrap();
				debug_assert!(ptr_eq(self, p));
				// TODO: can we do self = ?
				let mut replacement = NodeRef::Hot(hn_rc_cell);
				mem::swap(&mut replacement, self)
			}
		}
	}

	/// Reassigns this node ref to the HotNode referred to by the given HotHandle.
	/// The given HotHandle must be a modified copy of the HotNode inside this NodeRef (verified with debug assertions).
	/// The difference between this fn and reassign is that this fn is intended for creating new NodeRefs with
	/// modified nodes inside, whereas reassign is for read-modify-write cycles on one NodeRef.
	pub fn assign(&mut self, h: HotHandle) {
		match h {
			HotHandle::Existing(hn_ref) => {
				// Safety check: A HotHandle::Existing may only be reassigned to itself.
				if let &mut NodeRef::Hot(ref rc_rfc_hn) = self {
					debug_assert!(ptr_eq(rc_rfc_hn.deref().as_ptr(), hn_ref.deref() as *const _))
				} else {
					debug_assert!(false, "Cannot assign an existing HotNode to a warm NodeRef")
				}
			}
			HotHandle::New(h_nref, hn_rc_cell) => {
				debug_assert!(ptr_eq(self.hn_ptr(), h_nref.unwrap().hn_ptr()));
				// TODO: can we do self = ?
				let mut replacement = NodeRef::Hot(hn_rc_cell);
				mem::swap(&mut replacement, self)
			}
		}
	}

	/* Thermodynamics */

	/// Returns a hot NodeRef which may be modified, together with a reference to that node. May return self.
	pub fn heat<'a>(&'a self) -> (HotHandle<'a>, bool) {
		match self {
			&NodeRef::Hot(ref rc_cell_hn) => (HotHandle::Existing(rc_cell_hn.as_ref().borrow_mut()), false),
			&NodeRef::Warm(ref rc_hn) => {
				let newnode: HotNode = (*rc_hn).fork();
				(HotHandle::New(Some(self), Rc::new(RefCell::new(newnode))), true)
			}
		}
	}

	/// Immutes this NodeRef, recursively immuting its children.
	pub fn cool(&mut self) {
		// Bunch of footwork so we can modify ourselves in place without breaking mut safety.
		// Who knows if this optimizes correctly?
		let mut dummy = unsafe { mem::uninitialized() };
		let mut dummy2;
		mem::swap(self, &mut dummy);

		match dummy {
			NodeRef::Hot(rc_cell_hn) => {
				let mut hn = Rc::try_unwrap(rc_cell_hn).ok().unwrap().into_inner();
				hn.cool();
				dummy2 = NodeRef::Warm(Rc::new(hn));
			}
			NodeRef::Warm(x) => {
				dummy2 = NodeRef::Warm(x);
			}
		}

		mem::swap(self, &mut dummy2);
		mem::drop(dummy2);
	}

	/// Creates a mutable copy of this NodeRef.
	pub fn fork(&self) -> NodeRef {
		// Right now, all this does is throw if we're hot already.
		match self {
			&NodeRef::Hot(ref _x) => panic!("cannot fork a hot node"),
			&NodeRef::Warm(ref rc_) => NodeRef::Warm(rc_.clone()),
		}
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
