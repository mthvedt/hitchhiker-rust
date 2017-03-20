// //! A test library for btrees.
//
// use std::borrow::*;
// use std::collections::*;
//
// use rand::*;
//
// use data::*;
// use tree::btree::*;
//
// /// A Testable is anything that has a name, can be set up, and can be torn down.
// pub trait Testable {
// 	fn name() -> String;
// 	fn setup() -> Self;
// 	fn teardown(self) -> ();
// }
//
// impl Testable for PersistentBTree {
// 	fn name() -> String {
// 		String::from("PBTree")
// 	}
//
// 	fn setup() -> Self {
// 		Self::new()
// 	}
//
// 	fn teardown(self) {
// 		self.check_invariants();
// 	}
// }
//
// /// An empty struct Testable. Its name is "(n/a)".
// pub struct DummyTestable {}
//
// impl Testable for DummyTestable {
// 	fn name() -> String {
// 		String::from("(n/a)")
// 	}
//
// 	fn setup() -> Self {
// 		DummyTestable {}
// 	}
//
// 	fn teardown(self) {}
// }
//
// // Alas, this macro is verbose, but it's the best we have
// // (rust doesn't have gensym, dynamic idents, a stable testing interface, &c.)
// #[macro_export]
// macro_rules! deftests {
// 	// TODO: what is $tr for?
// 	{ $($testable:ty => { $($name:ident, $test:path,)* }, )* } => {
//         $(
//         	$(
//                 #[test]
//                 fn $name() {
// 					let mut o = <$testable as Testable>::setup();
// 					$test(&mut o);
// 					o.teardown();
//                 }
//             )*
//         )*
//     };
// }
//
// /// Convenience wrapper around a box of bytes.
// #[derive(PartialEq, Eq, Hash, PartialOrd, Ord)]
// pub struct ByteBox {
//     data: Box<[u8]>,
// }
//
// impl ByteBox {
//     // TODO: a 'ToBox' trait for Key
//     fn new<B: Borrow<[u8]>>(bytes: B) -> ByteBox {
//         ByteBox {
//             // TODO size check
//             data: SliceDatum::new(bytes.borrow()).box_copy(),
//         }
//     }
//
//     fn from_key<K: Key + ?Sized>(k: &K) -> ByteBox {
//         Self::new(k.bytes())
//     }
//
//     fn from_value<V: Datum>(v: &V) -> ByteBox {
//         ByteBox {
//             data: v.box_copy(),
//         }
//     }
// }
//
// impl Borrow<[u8]> for ByteBox {
//     fn borrow(&self) -> &[u8] {
//         self.data.borrow()
//     }
// }
//
// /// A pointer to a ByteBox. Used only in testing, to work around the fact that macros like deftests
// /// don't play well with lifetimes in certain contexts.
// pub struct ByteBoxRef {
// 	r: *const ByteBox,
// }
//
// impl ByteBoxRef {
// 	fn wrap(b: &ByteBox) -> ByteBoxRef {
// 		ByteBoxRef {
// 			r: b as *const _,
// 		}
// 	}
// }
//
// impl Borrow<ByteBox> for ByteBoxRef {
// 	fn borrow(&self) -> &ByteBox {
// 		unsafe { &*self.r }
// 	}
// }
//
// /// A ByteMap that wraps a HashMap of ByteBoxes, by value. Used as a reference test and reference benchmark.
// pub struct ByteHashMap {
// 	wrapped: HashMap<ByteBox, ByteBox>,
// }
//
// impl Testable for ByteHashMap {
// 	fn name() -> String {
// 		String::from("std hashmap")
// 	}
//
// 	fn setup() -> Self {
// 		ByteHashMap { wrapped: HashMap::new(), }
// 	}
//
// 	fn teardown(self) {}
// }
//
// impl ByteMap for ByteHashMap {
//     type GetDatum = ByteBox;
//     type Get = ByteBoxRef;
//
// 	fn get<K: Key + ?Sized>(&mut self, k: &K) -> Option<ByteBoxRef> {
// 		self.wrapped.get(&ByteBox::from_key(k)).map(ByteBoxRef::wrap)
// 	}
//
// 	fn check_invariants(&self) {
// 		// Do nothing, assume impl is correct
// 	}
// }
//
// impl MutableByteMap for ByteHashMap {
// 	fn insert<K: Key + ?Sized, V: Datum>(&mut self, k: &K, v: &V) {
// 		self.wrapped.insert(ByteBox::from_key(k), ByteBox::from_value(v));
// 	}
//
// 	fn delete<K: Key + ?Sized>(&mut self, k: &K) -> bool {
// 		self.wrapped.remove(&ByteBox::from_key(k)).is_some()
// 	}
// }
//
// /// A ByteMap that wraps a TreeMap of ByteBoxes, by value. Used as a reference test and reference benchmark.
// pub struct ByteTreeMap {
// 	wrapped: BTreeMap<ByteBox, ByteBox>,
// }
//
// impl Testable for ByteTreeMap {
// 	fn name() -> String {
// 		String::from("std btree")
// 	}
//
// 	fn setup() -> Self {
// 		ByteTreeMap { wrapped: BTreeMap::new(), }
// 	}
//
// 	fn teardown(self) {}
// }
//
// impl ByteMap for ByteTreeMap {
//     type GetDatum = ByteBox;
//     type Get = ByteBoxRef;
//
// 	fn get<K: Key + ?Sized>(&mut self, k: &K) -> Option<ByteBoxRef> {
// 		self.wrapped.get(&ByteBox::from_key(k)).map(ByteBoxRef::wrap)
// 	}
//
//
// 	fn check_invariants(&self) {
// 		// Do nothing, assume impl is correct
// 	}
// }
//
// impl MutableByteMap for ByteTreeMap {
// 	fn insert<K: Key + ?Sized, V: Datum>(&mut self, k: &K, v: &V) {
// 		self.wrapped.insert(ByteBox::from_key(k), ByteBox::from_value(v));
// 	}
//
// 	fn delete<K: Key + ?Sized>(&mut self, k: &K) -> bool {
// 		self.wrapped.remove(&ByteBox::from_key(k)).is_some()
// 	}
// }
//
// /// Generates an Rng from a usize. This fn exists so tests can be consistent in their choice of rng.
// pub fn rng(seed: usize) -> impl Rng {
// 	StdRng::from_seed(&[seed])
// }
//
// /// Returns one million 8-byte strings.
// pub fn random_byte_strings(seed: usize) -> Box<[[u8; 8]]> {
// 	let mut x = rng(seed);
// 	let mut v = Vec::<[u8; 8]>::new();
//
// 	for _ in 0..1000000 {
// 		let mut bytes = [0 as u8; 8];
// 		x.fill_bytes(bytes.borrow_mut());
// 		v.push(bytes);
// 	}
//
// 	v.into_boxed_slice()
// }
//
// /// Returns a byte string with expected length i + overflow, but not exceeding max.
// /// The distribution of lengths is exponentially decreasing. On the rare occasion that the chosen
// /// length exceeds the given max length, the byte string is truncated to max and the difference
// /// is returned.
// ///
// /// Returns the generated byte string and the truncated difference, or 0 if the string was not truncated.
// fn random_size_byte_string<R: Rng>(x: &mut R, i: usize, max: usize, overflow: isize) -> (Vec<u8>, isize) {
// 	let mut s = ((1.0 - x.next_f64()).ln() * -1.0 * (i as f64)) as isize + overflow;
// 	let mut overflow = 0;
//
// 	if s < 0 { // rare case
// 		s = 1;
// 		overflow = -1 - s;
// 	}
// 	if s > max as isize {
// 		overflow = s - max as isize;
// 		s = max as isize;
// 	}
//
// 	let mut r = Vec::new();
// 	r.reserve(s as usize);
// 	unsafe { r.set_len(s as usize) };
//
// 	x.fill_bytes(r.as_mut_slice());
//
// 	(r, overflow)
// }
//
// /// Returns 1000 random strings with average size 8000 bytes but not exceeding 64 binary kilobytes.
// pub fn random_big_byte_strings(seed: usize) -> Vec<Vec<u8>> {
// 	let mut x = rng(seed);
// 	let mut v = Vec::<Vec<u8>>::new();
// 	let mut overflow = 0;
//
// 	for _ in 0..1000 {
// 		let (x, new_overflow) = random_size_byte_string(&mut x, 8000, 65535, overflow);
// 		overflow = new_overflow;
// 		v.push(x);
// 	}
//
// 	v
// }
