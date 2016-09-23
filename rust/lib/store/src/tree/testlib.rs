extern crate rand;

use self::rand::*;

use std;
use std::collections::*;

use data::*;
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

// Alas, this macro is verbose, but it's the best we have
// (rust doesn't have gensym, dynamic idents, &c.)
// TODO: we can make this better/less verbose. See the bench macros in bench.rs
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
	wrapped: HashMap<Box<[u8]>, Value>,
}

impl Testable for ByteHashMap {
	fn name() -> String {
		String::from("byte hash map")
	}

	fn setup() -> Self {
		ByteHashMap { wrapped: HashMap::new(), }
	}

	fn teardown(self) {}
}

impl<'a> ByteMap for ByteHashMap {
	type D = Value;

	fn insert<K: Key, V: Datum>(&mut self, k: K, v: &V) {
		self.wrapped.insert(k.box_copy(), Value::new(v));
	}

	/// This is mutable because gets may introduce read conflicts, and hence mutate the underlying datastructure.
	fn get<K: Key>(&mut self, k: K) -> Option<&Value> {
		self.wrapped.get(&k.box_copy())
	}

	fn delete<K: Key>(&mut self, k: K) -> bool {
		self.wrapped.remove(&k.box_copy()).is_some()
	}
}

pub fn rng(seed: usize) -> impl Rng {
	StdRng::from_seed(&[seed])
}

pub fn random_byte_strings(seed: usize) -> Box<[[u8; 8]]> {
	let mut x = rng(seed);
	let mut v = Vec::<[u8; 8]>::new();

	for _ in 0..1000000 {
		let rnum = x.next_u64();
		let bytes: [u8; 8] = unsafe { std::mem::transmute(rnum) };
		v.push(bytes);
	}

	v.into_boxed_slice()
}

// pub trait ToBytes<'a> {
// 	type B: Borrow<[u8]>;
// 	fn to_bytes(&'a self) -> Self::B;
// }

// impl<'a> ToBytes<'a> for [u8] {
// 	type B = &'a [u8];

// 	fn to_bytes(&'a self) -> &'a [u8] {
// 		self
// 	}
// }

// impl<'a> ToBytes<'a> for &'a [u8] {
// 	type B = &'a [u8];

// 	fn to_bytes(&'a self) -> &'a [u8] {
// 		*self
// 	}
// }

// impl<'a> ToBytes<'a> for Box<[u8]> {
// 	type B = &'a [u8];

// 	fn to_bytes(&'a self) -> &'a [u8] {
// 		&**self
// 	}
// }

// impl<'a, D> ToBytes<'a> for D where D: Datum {
// 	type B = Box<[u8]>;

// 	fn to_bytes(&'a self) -> Self::B {
// 		self.box_copy()
// 	}
// }

// impl<'a> ToBytes<'a> for [u8] {
// 	fn to_bytes(&'a self) -> &'a [u8] {
// 		self
// 	}
// }

// impl<'a> ToBytes<'a> for &'a [u8] {
// 	fn to_bytes(&'a self) -> &'a [u8] {
// 		*self
// 	}
// }

// impl<'a> ToBytes<'a> for Box<[u8]> {
// 	fn to_bytes(&'a self) -> &'a [u8] {
// 		&**self
// 	}
// }

// impl<'a> ToBytes<'a> for D where D: Datum {
// 	fn to_bytes(&'a self) -> &'a [u8] {
// 		&**self
// 	}
// }

// pub trait ToByteComparison<B> {
// 	fn compare_bytes(&self, b: &B) -> bool;
// }

// impl<A, B> ToByteComparison<B> for A where for<'a> A: ToBytes<'a>, for<'b> B: ToBytes<'b> {
// 	fn compare_bytes(&self, b: &B) -> bool {
// 		self.to_bytes() == b.to_bytes()
// 	}
// }

// impl<A, B> ToByteComparison<Option<B>> for Option<A>
// where for<'a> A: ToBytes<'a>, for<'b> B: ToBytes<'b>
// {
// 	fn compare_bytes(&self, b: &Option<B>) -> bool {
// 		self.as_ref().map(ToBytes::to_bytes).as_ref().map(Borrow::borrow)
// 		== b.as_ref().map(ToBytes::to_bytes).as_ref().map(Borrow::borrow)
// 	}
// }

// fn compare_opt<A, B>(a: &Option<A>, b: &Option<B>) -> bool
// where for<'a> A: ToBytes<'a>, for<'b> B: ToBytes<'b>
// {
// 	a.as_ref().map(ToBytes::to_bytes) == b.as_ref().map(ToBytes::to_bytes)
// }
