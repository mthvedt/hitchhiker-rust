extern crate rand;

use self::rand::*;

use std::borrow::BorrowMut;
use std::collections::*;

use data::*;
use data::slice::*;
use tree::btree::*;

pub trait Testable {
	fn name() -> String;
	fn setup() -> Self;
	fn teardown(mut self) -> ();
}

impl Testable for BTree {
	fn name() -> String {
		String::from("BTree")
	}

	fn setup() -> Self {
		Self::new()
	}

	fn teardown(self) {}
}

impl Testable for PersistentBTree {
	fn name() -> String {
		String::from("PBTree")
	}

	fn setup() -> Self {
		Self::new()
	}

	fn teardown(self) {}
}

/// A Testable that does nothing. Useful for using the defbench macro for one-offs.
pub struct DummyTestable {}

impl Testable for DummyTestable {
	fn name() -> String {
		String::from("(n/a)")
	}

	fn setup() -> Self {
		DummyTestable {}
	}

	fn teardown(self) {}
}

// Alas, this macro is verbose, but it's the best we have
// (rust doesn't have gensym, dynamic idents, a stable testing interface, &c.)
#[macro_export]
macro_rules! deftests {
	// TODO: what is $tr for?
	{ $($testable:ty: $tr:ty => { $($name:ident, $test:path,)* }, )* } => {
        $(
        	$(
                #[test]
                fn $name() {
					let mut o = <$testable as Testable>::setup();
					$test(&mut o);
					o.teardown();
                }
            )*
        )*
    };
}

/// A ByteMap impl that boxes references into a HashMap. Of course boxing references is a little slow,
/// but it's "fair" in the sense a real DB will need to allocate and copy *something*.
/// We also have benchmarks for raw byte string references in the bench/ binary.
pub struct ByteHashMap {
	wrapped: HashMap<ByteBox, ByteBox>,
}

impl Testable for ByteHashMap {
	fn name() -> String {
		String::from("std hashmap")
	}

	fn setup() -> Self {
		ByteHashMap { wrapped: HashMap::new(), }
	}

	fn teardown(self) {}
}

impl ByteMap for ByteHashMap {
	type D = ByteBox;

	fn insert<K: Key + ?Sized, V: Datum>(&mut self, k: &K, v: &V) {
		self.wrapped.insert(ByteBox::from_key(k), ByteBox::from_value(v));
	}

	fn get<K: Key + ?Sized>(&mut self, k: &K) -> Option<&ByteBox> {
		self.wrapped.get(&ByteBox::from_key(k))
	}

	fn delete<K: Key + ?Sized>(&mut self, k: &K) -> bool {
		self.wrapped.remove(&ByteBox::from_key(k)).is_some()
	}
}

pub struct ByteTreeMap {
	wrapped: BTreeMap<ByteBox, ByteBox>,
}

impl Testable for ByteTreeMap {
	fn name() -> String {
		String::from("std btree")
	}

	fn setup() -> Self {
		ByteTreeMap { wrapped: BTreeMap::new(), }
	}

	fn teardown(self) {}
}

impl ByteMap for ByteTreeMap {
	type D = ByteBox;

	fn insert<K: Key + ?Sized, V: Datum>(&mut self, k: &K, v: &V) {
		self.wrapped.insert(ByteBox::from_key(k), ByteBox::from_value(v));
	}

	fn get<K: Key + ?Sized>(&mut self, k: &K) -> Option<&ByteBox> {
		self.wrapped.get(&ByteBox::from_key(k))
	}

	fn delete<K: Key + ?Sized>(&mut self, k: &K) -> bool {
		self.wrapped.remove(&ByteBox::from_key(k)).is_some()
	}
}

pub fn rng(seed: usize) -> impl Rng {
	StdRng::from_seed(&[seed])
}

/// One million 8-byte strings.
pub fn random_byte_strings(seed: usize) -> Box<[[u8; 8]]> {
	let mut x = rng(seed);
	let mut v = Vec::<[u8; 8]>::new();

	for _ in 0..1000000 {
		let mut bytes = [0 as u8; 8];
		x.fill_bytes(bytes.borrow_mut());
		v.push(bytes);
	}

	v.into_boxed_slice()
}

/// Returns a byte string with average size i + overflow, not exceeding max, with exponential decay distribution.
///
/// Overflow is a 'carry' for when the byte strings exceed max (or are less than 1)
/// and need to have their size adjsuted. The adjustment is returned, and can be used
/// to 'carry' over to the size of the next byte string, so that the average size remains the same.
fn random_size_byte_string<R: Rng>(x: &mut R, i: usize, max: usize, overflow: isize) -> (Vec<u8>, isize) {
	let mut s = ((1.0 - x.next_f64()).ln() * -1.0 * (i as f64)) as isize + overflow;
	let mut overflow = 0;

	if s < 0 { // rare case
		s = 1;
		overflow = -1 - s;
	}
	if s > max as isize {
		overflow = s - max as isize;
		s = max as isize;
	}

	let mut r = Vec::new();
	r.reserve(s as usize);
	unsafe { r.set_len(s as usize) };

	x.fill_bytes(r.as_mut_slice());

	(r, overflow)
}

/// 1k byte strings with average size 8000 bytes, not exceeding 64k.
pub fn random_big_byte_strings(seed: usize) -> Vec<Vec<u8>> {
	let mut x = rng(seed);
	let mut v = Vec::<Vec<u8>>::new();
	let mut overflow = 0;

	for _ in 0..1000 {
		let (x, new_overflow) = random_size_byte_string(&mut x, 8000, 65535, overflow);
		overflow = new_overflow;
		// println!("{} {}", x.len(), overflow as i64);
		v.push(x);
	}

	v
}
