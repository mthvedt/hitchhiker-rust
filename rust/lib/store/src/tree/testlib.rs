extern crate rand;

use self::rand::*;

use std;

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

pub fn rng(seed: usize) -> impl Rng {
	StdRng::from_seed(&[seed])
}

pub fn random_byte_strings(seed: usize) -> Box<[[u8; 8]]> {
	let mut x = rng(seed);
	let mut v = Vec::<[u8; 8]>::new();

	for i in 0..1000000 {
		let rnum = x.next_u64();
		let bytes: [u8; 8] = unsafe { std::mem::transmute(rnum) };
		v.push(bytes);
	}

	v.into_boxed_slice()
}

// Alas, this macro is verbose, but it's the best we have
// (rust doesn't have gensym, dynamic idents, &c.)
// TODO: we can make this better/less verbose. See the bench macros in bench.rs
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
