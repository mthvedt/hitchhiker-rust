// use bytebuffer::ByteBuffer;

// use alloc::Scoped;
// use data::Range;
// use tdfuture::Waiter;
// use traits::{Source, Sink};

// enum Void {}

// pub struct VoidFuture<Item, Error> {
//     v: Void,
//     phantom: PhantomData<(Item, Error)>,
// }

// impl<Item, Error> Future for VoidFuture<Item, Error> {
//     type Item = Item;
//     type Error = Error;

//     fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
//         unreachable!()
//     }
// }

// /// A KvSink with only one supported key/value: the null one.
// pub struct NullKeyDummyKvSink {
//     buf: ByteBuffer,
// }

// impl NullKeyDummyKvSink {
//     pub fn new() -> Self {
//         NullKeyDummyKvSink {
//             buf: ByteBuffer::new(),
//         }
//     }

//     fn check_key<K: Scoped<[u8]>>(k: K) -> bool {
//         k.get().unwrap() == []
//     }
// }

// impl Source<[u8]> for NullKeyDummyKvSink {
//     type Get = Box<[u8]>;

//     fn get<K: Scoped<[u8]>, W: Waiter<Option<Self::Get>>>(&mut self, k: K, w: W) {
//         if Self::check_key(k) {
//             let len = self.buf.len();
//             w.recv_ok(Some(self.buf.read_bytes(len).into_boxed_slice()))
//         } else {
//             w.recv_ok(None)
//         }
//     }

//     fn subtree<K: Scoped<[u8]>>(&mut self, k: K) -> Self {
//         panic!("not yet implemented")
//         // panic!("key not supported in this test class");
//     }

//     fn subrange(&mut self, range: Range) -> Self {
//         panic!("not yet implemented")
//     }
// }
