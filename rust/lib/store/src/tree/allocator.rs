/*
Allocator design...

Pointer source.
Persistent pointers, transient pointers, weak pointers.
How to do persistent pointers?

Transient and weak are easy: just have them allocated from an arena.

Arenas are thread unsafe.
Recall that our usecase is on-disk, not in-memory; therefore, persistent nodes
are not shareable until turned into 'on-disk' nodes.

For speed, we actually need several typed allocators, perhaps using macros.
*/

// TODO: name?

//! Thread-unsafe arena allocators.

// trait Arena {
// 	type Handle<Arena = Self>: ArenaHandle;
// 	type Validator: ArenaValidator;

// 	fn handle(&self) -> Self::Handle;

// 	fn alloc<T>(&mut self, t: T) -> ArenaPtr<T>;
// }

// struct ArenaImpl {
// 	// Any frees of the Arena must happen-after this state is cleared.
// 	state: AtomicBool,
// }

// trait ArenaHandle {
// 	type Arena: Arena;

// 	fn get(&self) -> Self::Arena;
// }

// struct ArenaHandleImpl<A: Arena> {
// 	arena: Rc<A>,
// }

// trait ArenaValidator {
// 	// empty trait
// }

// /// ArenaPtrs represent shared data.
// trait ArenaPtr<T> {
// 	type Arena: Arena,

// 	fn deref(&self, a: <Self::Arena as Arena>::Validator) -> &T;
// }

// /// An ArenaPtrMut can be downgraded into an ArenaPtr, but not vice versa.
// trait ArenaPtrMut {

// }

// struct ArenaPtr<T> {

// }
