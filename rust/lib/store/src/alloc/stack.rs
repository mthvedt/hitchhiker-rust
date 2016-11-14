// //! Heterogeneous stacks in Rust.
// //!
// //! Heterogeneous stacks

// extern crate alloc;

// use std::cell::{Cell, RefCell};
// use std::marker;
// use std::mem::{align_of, size_of, transmute};
// use std::ptr;
// use self::alloc::heap::{allocate, deallocate};

trait StackPointer<T> {
    type NextT;
    type Next: StackPointer<Self::NextT>;

    fn get(&self) -> Option<&T> {
        self.get_mut().map(|t| &*t)
    }

    fn get_mut(&self) -> Option<&mut T>;

    fn pop(self) -> (T, Self::Next);

    fn drop(self) -> Self::Next;
}

// #[inline]
// fn round_up(base: usize, align: usize) -> usize {
//     (base.checked_add(align - 1)).unwrap() & !(align - 1)
// }

// pub struct StackPointer<T, P> where
// P: StackPointerLink
// {
//     prev: P
// }

// pub enum StackPointerLink {

// }

// /// A stack that just holds raw bytes, nothing special about it.
// struct DumbStack {
//     /// The next spot to be allocated.
//     ptr: Cell<*const u8>,

//     /// The end of the available stack space. A new chunk is allocated when the stack is exhausted.
//     end: Cell<*const u8>,

//     /// The first chunk.
//     first: RefCell<DumbChunk>,
// }

// struct DumbChunkHeader {
//     /// Pointer to the next chunk.
//     next: *mut DumbChunk,
//     /// The bytes available in this chunk (= size of this chunk minus space for this header).
//     len: usize,
// }

// /// A header chunk in a DumbStack. It contains a DumbChunkHeader, followed by raw bytes.
// struct DumbChunk {
//     bytes: *mut u8,
// }

// impl DumbChunk {
//     fn bytes_offset() -> usize {
//         max(size_of::<DumbChunkHeader>(), align_of::<DumbChunkHeader>());
//     }

//     /// Gets the len of this chunk from the chunk size.
//     fn calculate_len(alloc_size: usize) -> usize {
//         let header_size = max(size_of::<*mut DumbChunk>, align_of::<*mut DumbChunk>);
//         alloc_size - header_size;
//     }

//     /// Gets the size of this chunk from the chunk len.
//     fn calculate_size(len: usize) -> usize {
//         let header_size = max(size_of::<*mut DumbChunk>, align_of::<*mut DumbChunk>);
//         len + header_size
//     }

//     /// Creates a new chunk.
//     unsafe fn new(next: *mut DumbChunk, size: usize) -> DumbChunk {
//         let chunk_ptr = allocate(size, align_of::<DumbChunkHeader>()) as *mut u8;
//         if chunk.is_null() {
//             alloc::oom()
//         }
//         let chunk = DumbChunk {
//             bytes: chunk_ptr,
//         };

//         let len = calculate_len(capacity);
//         let header = chunk.header_mut();
//         header.next = next;
//         header.len = capacity;

//         chunk
//     }

//     /// Destroys this chunk.
//     unsafe fn destroy(&mut self) {
//         let next;
//         let size;
//         {
//             let header = (&*DumbChunk).header();
//             next = header.next;
//             size = calculate_size(header.len);
//         }

//         deallocate(self.bytes, size, align_of::<DumbChunk>());

//         if !next.is_null() {
//             (*next).destroy();
//         }
//     }

//     /// Returns a ref to the chunk header.
//     fn header(&self) -> &DumbChunkHeader {
//         unsafe {
//             transmute(self.bytes)
//         }
//     }

//     /// Returns a mut ref to the chunk header.
//     fn header_mut(&mut self) -> &mut DumbChunkHeader {
//         unsafe {
//             transmute(self.bytes)
//         }
//     }

//     // Returns a pointer to the beginning of the allocated space.
//     #[inline]
//     fn start(&self) -> *const u8 {
//         todo;
//         let this: *const TypedArenaChunk<T> = self;
//         unsafe {
//             transmute(round_up(this.offset(1) as usize, align_of::<T>()))
//         }
//     }

//     // Returns a pointer to the end of the allocated space.
//     #[inline]
//     fn end(&self) -> *const u8 {
//         todo;
//         unsafe {
//             let size = size_of::<T>().checked_mul(self.capacity).unwrap();
//             self.start().offset(size as isize)
//         }
//     }
// }

// impl DumbStack {
//     fn page_size() -> usize {
//         4096
//     }

//     pub fn new() -> DumbStack {
//         unsafe {
//             let chunk = DumbChunk::new(ptr::null_mut(), page_size());
//             DumbStack {
//                 ptr: Cell::new(chunk.start() as *const T),
//                 end: Cell::new(chunk.end() as *const T),
//                 first: RefCell::new(chunk),
//             }
//         }
//     }

//     /// Allocates an object in the `TypedArena`, returning a reference to it.
//     #[inline]
//     pub fn alloc(&self, object: T) -> &mut T {
//         if self.ptr == self.end {
//             self.grow()
//         }

//         let ptr: &mut T = unsafe {
//             let ptr: &mut T = transmute(self.ptr.clone());
//             ptr::write(ptr, object);
//             self.ptr.set(self.ptr.get().offset(1));
//             ptr
//         };

//         ptr
//     }

//     /// Grows the arena.
//     #[inline(never)]
//     fn grow(&self) {
//         todo; capacity;
//         unsafe {
//             let chunk = self.first.borrow_mut();
//             let new_len = chunk.header().len.checked_mul(2).unwrap();
//             let chunk = TypedArenaChunk::<T>::new(chunk, new_capacity);
//             self.ptr.set(chunk.start() as *const T);
//             self.end.set(chunk.end() as *const T);
//             *self.first.borrow_mut() = chunk
//         }
//     }
// }

// // impl<T> Drop for TypedArena<T> {
// //     fn drop(&mut self) {
// //         unsafe {
// //             (**self.first.borrow_mut()).destroy()
// //         }
// //     }
// // }

// // #[cfg(test)]
// // mod tests {
// //     extern crate test;
// //     use self::test::Bencher;
// //     use super::TypedArena;

// //     #[allow(dead_code)]
// //     struct Point {
// //         x: i32,
// //         y: i32,
// //         z: i32,
// //     }

// //     // #[test]
// //     // fn test_arena_alloc_nested() {
// //     //     struct Inner { value: u8 }
// //     //     struct Outer<'a> { inner: &'a Inner }
// //     //     enum EI<'e> { I(Inner), O(Outer<'e>) }

// //     //     struct Wrap<'a>(TypedArena<EI<'a>>);

// //     //     impl<'a> Wrap<'a> {
// //     //         fn alloc_inner<F:Fn() -> Inner>(&self, f: F) -> &Inner {
// //     //             let r: &EI = self.0.alloc(EI::I(f()));
// //     //             if let &EI::I(ref i) = r {
// //     //                 i
// //     //             } else {
// //     //                 panic!("mismatch");
// //     //             }
// //     //         }
// //     //         fn alloc_outer<F:Fn() -> Outer<'a>>(&self, f: F) -> &Outer {
// //     //             let r: &EI = self.0.alloc(EI::O(f()));
// //     //             if let &EI::O(ref o) = r {
// //     //                 o
// //     //             } else {
// //     //                 panic!("mismatch");
// //     //             }
// //     //         }
// //     //     }

// //     //     let arena = Wrap(TypedArena::new());

// //     //     let result = arena.alloc_outer(|| Outer {
// //     //         inner: arena.alloc_inner(|| Inner { value: 10 }) });

// //     //     assert_eq!(result.inner.value, 10);
// //     // }

// //     #[test]
// //     pub fn test_copy() {
// //         let arena = TypedArena::new();
// //         for _ in 0..100000 {
// //             arena.alloc(Point {
// //                 x: 1,
// //                 y: 2,
// //                 z: 3,
// //             });
// //         }
// //     }

// //     #[bench]
// //     pub fn bench_copy(b: &mut Bencher) {
// //         let arena = TypedArena::new();
// //         b.iter(|| {
// //             arena.alloc(Point {
// //                 x: 1,
// //                 y: 2,
// //                 z: 3,
// //             })
// //         })
// //     }

// //     #[bench]
// //     pub fn bench_copy_nonarena(b: &mut Bencher) {
// //         b.iter(|| {
// //             let _: Box<_> = box Point {
// //                 x: 1,
// //                 y: 2,
// //                 z: 3,
// //             };
// //         })
// //     }

// //     #[allow(dead_code)]
// //     struct Noncopy {
// //         string: String,
// //         array: Vec<i32>,
// //     }

// //     #[test]
// //     pub fn test_noncopy() {
// //         let arena = TypedArena::new();
// //         for _ in 0..100000 {
// //             arena.alloc(Noncopy {
// //                 string: "hello world".to_string(),
// //                 array: vec!( 1, 2, 3, 4, 5 ),
// //             });
// //         }
// //     }

// //     #[bench]
// //     pub fn bench_noncopy(b: &mut Bencher) {
// //         let arena = TypedArena::new();
// //         b.iter(|| {
// //             arena.alloc(Noncopy {
// //                 string: "hello world".to_string(),
// //                 array: vec!( 1, 2, 3, 4, 5 ),
// //             })
// //         })
// //     }

// //     #[bench]
// //     pub fn bench_noncopy_nonarena(b: &mut Bencher) {
// //         b.iter(|| {
// //             let _: Box<_> = box Noncopy {
// //                 string: "hello world".to_string(),
// //                 array: vec!( 1, 2, 3, 4, 5 ),
// //             };
// //         })
// //     }
// // }
