//! Multiple related kinds of 'fat' tagged pointers to different kinds of nodes.

use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::mem;
use std::ops::{Deref, DerefMut};

use tree::counter::*;
use tree::memnode::*;

/// A handle to a hot node which can be quickly dereferenced. Note that it's lifetimed--
/// HotHandles are intended to be ephemeral.
// TODO: HotHandle -> TransientRef
pub enum HotHandle {
    Existing(Weak<RefCell<MemNode>>),
    New(Rc<RefCell<MemNode>>),
}

impl HotHandle {
    /// Do something to the referenced MemNode.
    pub fn apply_mut<F, R> (&mut self, f: F) -> R where F: FnOnce(&mut MemNode) -> R
    {
        match *self {
            // Same call, different objects. Necessary because of the monomorphism restriction.
            HotHandle::Existing(ref mut w_rfc_hn) => {
                // borrow checker tricks
                let strong = w_rfc_hn.upgrade().unwrap();
                let r = f(strong.borrow_mut().deref_mut());
                r
            }
            HotHandle::New(ref mut rc_rfc_hn) => f(Rc::get_mut(rc_rfc_hn).unwrap().borrow_mut().deref_mut()),
        }
    }
}

/// A fat pointer to a Node or node address. Used externally by tree algorithms.
/// These NodeRefs are 'weak' and must be reloaded after context switches.
#[derive(Clone)]
pub enum NodeRef {
    Transient(Weak<RefCell<MemNode>>),
    Persistent(Weak<PersistentNode>),
}

impl NodeRef {
    pub fn upgrade(&self) -> FatNodeRef {
        match *self {
            NodeRef::Transient(ref rc_rfc_hn) => FatNodeRef::Transient(rc_rfc_hn.upgrade().unwrap()),
            NodeRef::Persistent(ref rc_pn) => FatNodeRef::Persistent(rc_pn.upgrade().unwrap()),
        }
    }

    pub fn apply<F, R> (&self, f: F) -> R where
    F: Fn(&MemNode) -> R
    {
        self.upgrade().apply(f)
    }

    // TODO: figure out the 'apply' story.
    pub fn apply_persistent<F, R> (&self, f: F) -> R where
    F: Fn(&PersistentNode) -> R
    {
        self.upgrade().apply_persistent(f)
    }

    /// Returns a hot NodeRef which may be modified, together with a reference to that node. May return self.
    pub fn heat(&self) -> (HotHandle, bool) {
        match *self {
            NodeRef::Transient(ref rc_rfc_hn) => (HotHandle::Existing(rc_rfc_hn.clone()), false),
            NodeRef::Persistent(ref rc_pn) => {
                let newnode = rc_pn.upgrade().unwrap().deref().fork();
                (HotHandle::New(Rc::new(RefCell::new(newnode))), true)
            }
        }
    }
}

// TODO move to a different .rs file
pub struct PersistentNode {
    // TODO fields should be private
    txid: Counter,
    // We recycle MemNode as persistent nodes.
    pub node: MemNode,
}

impl PersistentNode {
    fn fork(&self) -> MemNode {
        self.node.fork()
    }

    pub fn txid(&self) -> Counter {
        self.txid
    }
}

/// A fat pointer to a Node. If hot, may pin underlying unique or shared resources.
/// These are never invalidated after context switches, and used internally by MemNodes.
// TODO: FatNodeRef -> HotNodeRef? RcNodeRef? StrongNodeRef?
// TODO: move all this shit into NodeRef, move FatNodeRef into MemNode
// Doesn't implement Clone. Although cloneable, we want to disallow clones of the transient variant.
pub enum FatNodeRef {
    // Hot(RefCell<Rc<MemNode>>),
    // TODO shouldn't be RC
    Transient(Rc<RefCell<MemNode>>),
    Persistent(Rc<PersistentNode>),
}

impl FatNodeRef {
    /* Constructors */
    // TODO shouldn't need
    pub fn new_transient(n: MemNode) -> Self {
        FatNodeRef::Transient(Rc::new(RefCell::new(n)))
    }

    /* Accessors */
    pub fn apply<F, R>(&self, f: F) -> R where
    F: FnOnce(&MemNode) -> R
    {
        match *self {
            FatNodeRef::Transient(ref rc_rfc_hn) => f(rc_rfc_hn.deref().borrow().deref()),
            FatNodeRef::Persistent(ref rc_pn) => f(&rc_pn.deref().node),
        }
    }

    pub fn apply_persistent<F, R>(&self, f: F) -> R where
    F: FnOnce(&PersistentNode) -> R
    {
        match *self {
            FatNodeRef::Transient(_) => panic!("node is not persistent"),
            FatNodeRef::Persistent(ref rc_pn) => f(&rc_pn.deref()),
        }
    }

    pub fn noderef(&self) -> NodeRef {
        match *self {
            FatNodeRef::Transient(ref rc_) => NodeRef::Transient(Rc::downgrade(&rc_)),
            FatNodeRef::Persistent(ref rc_) => NodeRef::Persistent(Rc::downgrade(&rc_)),
        }
    }

    /// Reassigns this node ref to the MemNode referred to by the given HotHandle.
    /// The given HotHandle must point to the same MemNode as this FatNodeRef.
    // TODO: better implementations for this
    pub fn reassign(&mut self, h: HotHandle) {
        match h {
            HotHandle::Existing(_) => {
                // TODO: impl a safety check?

                // Safety check: A HotHandle::Existing may only be reassigned to itself.
                // (Safety check disabled because perf consequences)
                // if let &mut FatNodeRef::Transient(ref rc_rfc_hn) = self {
                //     let tgt = rc_rfc_hn.deref().as_ptr();
                //     let src = hn_ref.deref() as *const _;
                //     debug_assert!(ptr_eq(tgt, src),
                //         "Mismatch in node reassignment: target {:p}, source {:p}", tgt, src);
                // } else {
                //     debug_assert!(false, "Cannot assign an existing MemNode to a persistent FatNodeRef")
                // }
            }
            HotHandle::New(hn_rc_cell) => {
                // TODO: can we do self = ?
                let mut replacement = FatNodeRef::Transient(hn_rc_cell);
                mem::swap(&mut replacement, self)
            }
        }
    }

    /// Immutes this NodeRef, recursively immuting its children.
    pub fn immute(&mut self, txid: Counter) {
        // Bunch of footwork so we can modify ourselves in place without breaking mut safety.
        // Who knows if this optimizes correctly?
        // TODO: use mem::replace
        let mut oldself = unsafe { mem::uninitialized() };
        let mut newself;
        mem::swap(self, &mut oldself);
        // now self is uninitialized

        // destroys oldself
        match oldself {
            FatNodeRef::Transient(rc_cell_hn) => {
                let mut hn = Rc::try_unwrap(rc_cell_hn).ok().unwrap().into_inner();
                hn.immute(txid);
                newself = FatNodeRef::Persistent(Rc::new(PersistentNode {
                    txid: txid,
                    node: hn,
                }));
            }
            FatNodeRef::Persistent(_x) => {
                newself = FatNodeRef::Persistent(_x);
            }
        }

        mem::swap(self, &mut newself);
        // now newself is uninitialized
        mem::forget(newself);
    }

    pub fn shallow_clone(&self) -> FatNodeRef {
        match self {
            &FatNodeRef::Transient(ref _x) => panic!("cannot shallow_clone a hot node"),
            &FatNodeRef::Persistent(ref _x) => FatNodeRef::Persistent(_x.clone()),
        }
    }

    // // For the edge case where head has 1 child.
    // pub fn disown_only_child(&mut self) -> NodePtr {
    //  if self.bucket_count() != 0 || self.is_leaf() {
    //      panic!("called disown_only_child when buckets are present")
    //  }
    //  let mut r = NodePtr::empty();
    //  mem::swap(&mut r, &mut self.children[0]);
    //  r
    // }

    // /// Postcondition: May leave this node deficient.
    // pub fn delete(&mut self, k: &[u8]) -> bool {
    //  // Unlike in insert, we rebalance *after* delete.
    //  match self.find(k) {
    //      Ok(idx) => {
    //          if self.is_leaf() {
    //              Some(Self::cool(self.to_hot().delete_bucket(idx)));
    //              true
    //          } else {
    //              if idx > 0 {
    //                  // get leftmost descendant from right child
    //                  let new_child = self.get_child(idx + 1).heat();
    //                  let new_bucket = new_child.yank_leftmost_bucket();
    //                  let hn = self.heat();
    //                  hn.replace_bucket(idx, new_bucket);
    //                  hn.replace_child(idx + 1, new_child);
    //                  hn.check_deficient_child(idx + 1);
    //                  true
    //              } else {
    //                  // get rightmost descendant from left child
    //                  let new_child = self.get_child(idx).heat();
    //                  let new_bucket = new_child.yank_rightmost_bucket();
    //                  r.replace_bucket(idx, new_bucket);
    //                  r.replace_child(idx, new_child);
    //                  r.check_deficient_child(idx);
    //                  true
    //              }
    //          }
    //      },
    //      Err(idx) => if !self.is_leaf() {
    //          match self.get_child_mut(idx).delete(k) {
    //              Some(newchild) => {
    //                  let r = self.heat();
    //                  r.check_deficient_child(idx);
    //                  Some(Self::cool(r))
    //              }
    //              None => None
    //          }
    //      } else {
    //          None
    //      },
    //  }
    // }

    fn is_transient(&self) -> bool {
        match *self {
            FatNodeRef::Transient(_) => true,
            FatNodeRef::Persistent(_) => false,
        }
    }

    pub fn check_invariants(&self) {
        self.check_invariants_helper(None, None, self.is_transient(), true)
    }

    pub fn check_invariants_helper(&self, parent_lower_bound: Option<&[u8]>, parent_upper_bound: Option<&[u8]>,
        is_transient: bool, recurse: bool) {

        if !is_transient && self.is_transient() {
            panic!("failed invariant: child of immutable node is hot");
        } else {
            self.noderef().apply(|n| n.check_invariants_helper(parent_lower_bound, parent_upper_bound,
                self.is_transient(), recurse));
        }
    }
}
