use std::mem;
use std::rc::Rc;
use std::cell::RefCell;
use std::ops::Deref;

use data::{ByteRc, Datum};

use tree::btree::*;
use tree::bucket::*;
use tree::counter::*;
use tree::memnode::*;
use tree::noderef::*;
use tree::util::*;

// TODO what this really is is a reference head. Does it deserve its own class?
pub struct NodePtr {
	v: Option<FatNodeRef>,
}

// TODO this class is redundant
impl NodePtr {
	pub fn new_transient(p: MemNode) -> Self {
		Self::wrap(FatNodeRef::new_transient(p))
	}

	pub fn wrap(p: FatNodeRef) -> Self {
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

	pub fn deref(&self) -> Option<&FatNodeRef> {
		self.v.as_ref()
	}

	pub fn unwrap(&self) -> &FatNodeRef {
		self.v.as_ref().unwrap()
	}

	pub fn deref_mut(&mut self) -> Option<&mut FatNodeRef> {
		self.v.as_mut()
	}

	pub fn unwrap_mut(&mut self) -> &mut FatNodeRef {
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
			None => Self::new_transient(MemNode::new_from_one(Bucket::new(k, v))),
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
