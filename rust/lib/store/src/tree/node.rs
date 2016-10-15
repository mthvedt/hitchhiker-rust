// //! node.rs
// //! The node trait together with node operations.

// /// An operation that can be applied to a node. Primarily used for two things:
// /// * A poor man's polymorphism mechanism, via the apply_op and apply_op_mut methods on Node.
// /// It is necessary to use Operations for this because HKT closures are not supported by Rust.
// /// * A means of being able to track and record changes for debugging purposes. (Not implemented yet.)
// ///
// /// As an aside, we'd really like to make these generic over any trait, but Rust doesn't allow
// /// generics or associated types with trait bounds.
// trait NodeOp: Clone {
//     type Result;

//     fn apply<N: Node + ?Sized>(&self, n: &N) -> Self::Result where Self: Sized;

//     /// Virtual apply method, so that NodeOps can be used polymorphically.
//     fn apply_virt(&self, n: &Node) -> Self::Result {
//     	self.apply(n)
//     }
// }

// trait NodeOpMut: Clone {
//     type Target;
//     type Result;

//     fn apply<N: Node + ?Sized>(&self, n: &mut N) -> Self::Result;

//     /// Virtual apply method, so that NodeOpMuts can be used polymorphically.
//     fn apply_virt(&self, n: &mut Node) -> Self::Result {
//     	self.apply(n)
//     }
// }

// macro_rules! defop_struct {
//     () => ()
// }

// macro_rules! defops {
//     () => ()
// }

// #[derive(Clone)]
// struct GetOp(u16);

// impl NodeOp for GetOp {
// 	type Result = ();

//     fn apply<N: Node + ?Sized>(&self, n: &N) -> Self::Result where Self: Sized {
//     	n.get(self.0)
//     }
// }

// trait Node {
//     /// Applies an operation to a Node.
//     fn apply_op<O: NodeOp>(&self, op: &O) -> O::Result where Self: Sized {
//         op.apply(self)
//     }

//     fn apply_op_mut<O: NodeOpMut>(&mut self, op: &O) -> O::Result where Self: Sized  {
//         op.apply(self)
//     }

//     /// TODO can this be a u8?
//     fn get(&self, idx: u16);
// }

// trait AbstractNode: Node + Sized {
// 	fn get(&self, idx: u16) {
// 		self.apply_op(&GetOp(idx))
// 	}
// }
