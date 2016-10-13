use std::mem;
use std::rc::Rc;
use std::cell::RefCell;
use std::ops::Deref;

use data::{ByteRc, Datum};

use tree::btree::*;
use tree::counter::*;
use tree::hotnode::*;
use tree::util::*;

// TODO: this doesn't deserve its own module. Move to HotNode?

pub struct NodePtr {
	v: Option<FatNode>,
}

// TODO this class is redundant
impl NodePtr {
	pub fn new_transient(p: HotNode) -> Self {
		Self::wrap(FatNode::new_transient(p))
	}

	pub fn wrap(p: FatNode) -> Self {
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

	pub fn deref(&self) -> Option<&FatNode> {
		self.v.as_ref()
	}

	pub fn unwrap(&self) -> &FatNode {
		self.v.as_ref().unwrap()
	}

	pub fn deref_mut(&mut self) -> Option<&mut FatNode> {
		self.v.as_mut()
	}

	pub fn unwrap_mut(&mut self) -> &mut FatNode {
		self.v.as_mut().unwrap()
	}

	// Immtues this NodePtr, recurisvely immuting its children.
	pub fn cool(&mut self, txid: Counter) {
		match self.v.as_mut() {
			Some(noderef) => noderef.cool(txid),
			None => (),
		}
	}

	pub fn shallow_clone(&self) -> Self {
		match self.v.as_ref() {
			Some(noderef) => Self::wrap(noderef.shallow_clone()),
			None => Self::empty(),
		}
	}

	pub fn insert<D: Datum>(self, k: &[u8], v: &D) -> Self {
		match self.v {
			Some(noderef) => Self::wrap(noderef.handle().insert(k, v)),
			None => Self::new_transient(HotNode::new_from_one(k, v)),
		}
	}

	pub fn get(&mut self, k: &[u8]) -> Option<ByteRc> {
		match self.v.as_ref() {
			Some(nref) => (*nref).handle().get(k),
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
			Some(nref) => nref.check_invariants(),
			None => (),
		}
	}
}

#[derive(Clone)]
pub struct PersistentNode {
	// TODO this field should be private
	pub v: Rc<(Counter, HotNode)>,
}

impl PersistentNode {
	fn fork_hot(&self) -> HotNode {
		self.v.deref().1.fork()
	}
}

/// A fat pointer to a Node. May pin underlying unique or shared resources.
pub enum FatNode {
	// Hot(RefCell<Rc<HotNode>>),
	// TODO shouldn't be RC
	Transient(Rc<RefCell<HotNode>>),
	Persistent(PersistentNode),
}

use self::FatNode::*;

impl FatNode {

	/* Constructors */
	// TODO shouldn't need
	pub fn new_transient(n: HotNode) -> Self {
		Transient(Rc::new(RefCell::new(n)))
	}

	/* Accessors */
	pub fn apply<F, R> (&self, f: F) -> R where
	F: Fn(&HotNode) -> R
	{
		match self {
			// Same call, different objects. Necessary because of the monomorphism restriction.
			&Transient(ref rfc_rc_hn) => f(rfc_rc_hn.deref().borrow().deref()),
			&Persistent(ref pnode) => f(&pnode.v.deref().1),
		}
	}

	// TODO: delete me.
	pub fn handle(&self) -> NodeHandle {
		match self {
			&Transient(ref rc_) => NodeHandle::Hot(Rc::downgrade(&rc_)),
			&Persistent(ref pnode) => NodeHandle::Warm(Rc::downgrade(&pnode.v)),
		}
	}

	/// Reassigns this node ref to the HotNode referred to by the given HotHandle.
	/// The given HotHandle must point to the same HotNode as this FatNode.
	// TODO: better implementations for this
	pub fn reassign(&mut self, h: HotHandle) {
		match h {
			HotHandle::Existing(hn_ref) => {
				// Safety check: A HotHandle::Existing may only be reassigned to itself.
				if let &mut Transient(ref rc_rfc_hn) = self {
					let tgt = rc_rfc_hn.deref().as_ptr();
					let src = hn_ref.deref() as *const _;
					debug_assert!(ptr_eq(tgt, src),
						"Mismatch in node reassignment: target {:p}, source {:p}", tgt, src);
				} else {
					debug_assert!(false, "Cannot assign an existing HotNode to a persistent FatNode")
				}
			}
			HotHandle::New(h_nref, hn_rc_cell) => {
				// TODO: can we do self = ?
				let mut replacement = Transient(hn_rc_cell);
				mem::swap(&mut replacement, self)
			}
		}
	}

	/* Thermodynamics */

	/// Returns a hot NodeRef which may be modified, together with a reference to that node. May return self.
	pub fn heat<'a>(&'a self) -> (HotHandle<'a>, bool) {
		match self {
			&Transient(ref rc_cell_hn) => (HotHandle::Existing(rc_cell_hn.as_ref().borrow_mut()), false),
			&Persistent(ref pnode) => {
				let newnode: HotNode = pnode.fork_hot();
				(HotHandle::New(Some(self), Rc::new(RefCell::new(newnode))), true)
			}
		}
	}

	/// Immutes this NodeRef, recursively immuting its children.
	pub fn cool(&mut self, txid: Counter) {
		// Bunch of footwork so we can modify ourselves in place without breaking mut safety.
		// Who knows if this optimizes correctly?
		let mut oldself = unsafe { mem::uninitialized() };
		let mut newself;
		mem::swap(self, &mut oldself);
		// now self is uninitialized

		// destroys oldself
		match oldself {
			Transient(rc_cell_hn) => {
				let mut hn = Rc::try_unwrap(rc_cell_hn).ok().unwrap().into_inner();
				hn.cool(txid);
				newself = Persistent(PersistentNode {
					v: Rc::new((txid, hn)),
				});
			}
			Persistent(x) => {
				newself = Persistent(x);
			}
		}

		mem::swap(self, &mut newself);
		// now newself is uninitialized
		mem::forget(newself);
	}

	pub fn shallow_clone(&self) -> FatNode {
		match self {
			&Transient(ref _x) => panic!("cannot shallow_clone a hot node"),
			&Persistent(ref pnode) => Persistent(pnode.clone()),
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

	fn is_transient(&self) -> bool {
		match self {
			&Transient(_) => true,
			&Persistent(_) => false,
		}
	}

	fn check_invariants(&self) {
		self.check_invariants_helper(None, None, self.is_transient(), true)
	}

	pub fn check_invariants_helper(&self, parent_lower_bound: Option<&[u8]>, parent_upper_bound: Option<&[u8]>,
		is_transient: bool, recurse: bool) {

		if !is_transient && self.is_transient() {
			panic!("failed invariant: child of immutable node is hot");
		} else {
			self.apply(|n| n.check_invariants_helper(parent_lower_bound, parent_upper_bound, self.is_transient(),
				recurse));
		}
	}
}
