extern crate rand;
extern crate test;
extern crate time;

use std;
use std::convert::TryFrom;
use std::vec::Vec;

use self::rand::*;
use self::time::*;
use self::test::black_box;

use data::*;
use tree::btree::{ByteMap, ByteTree, BTree};

trait Testable {
	fn setup() -> Self;
	fn teardown(mut self) -> ();
	fn name() -> &'static str;
}

impl Testable for BTree {
	fn setup() -> Self {
		Self::new()
	}

	fn teardown(self) {}

	fn name() -> &'static str {
		&"BTree"
	}
}

enum BenchResult {
	Ok(Duration, u64),
	Fail(String),
}

struct Bencher {
	result: BenchResult,
}

impl Bencher {
	fn bench<T, F>(&mut self, count: u64, f: F) where F: FnOnce() -> T {
		let start = PreciseTime::now();

		let t = f();

		let end = PreciseTime::now();

		test::black_box(&t);

		self.result = BenchResult::Ok(start.to(end), count);
	}
}

trait Verifier {
	fn run_update<F>(f: F) where F: FnOnce();
	fn verify<F>(message: &Fn() -> String, f: F) where F: FnOnce() -> bool;
	fn verify_custom<F>(f: F) where F: FnOnce() -> Option<String>;
}

struct NullVerifier {}

impl Verifier for NullVerifier {
	fn run_update<F>(f: F) where F: FnOnce() {}
	fn verify<F>(message: &Fn() -> String, f: F) where F: FnOnce() -> bool {}
	fn verify_custom<F>(f: F) where F: FnOnce() -> Option<String> {}
}

struct RealVerifier {}

impl Verifier for RealVerifier {
	fn run_update<F>(f: F) where F: FnOnce() { f() }

	fn verify<F>(message: &Fn() -> String, f: F) where F: FnOnce() -> bool {
		if !(f()) {
			panic!(message());
		}
	}

	fn verify_custom<F>(f: F) where F: FnOnce() -> Option<String> {
		match f() {
			Some(s) => panic!(s),
			None => (),
		}
	}
}

// /// What we really want is a Haskell-like typeclass, like VerifiableBenchmark<T> where T is a trait.
// /// But in Rust, T must be a concrete type. We can put trait bounds on fns, however, so benchmarks
// /// are of type f<t: T>() -> BenchInfo where T extends Testable. We use functions and macros to dress
// /// it up and make it easier.
// struct BenchInfo {
// 	name: String,
// 	benchmark: Box<Fn(&mut Bencher)>,
// 	verifymark: Box<(Fn())>,
// }

/// A benchmarkable closure of some kind.
trait Benchable {
	fn name() -> String;
	fn bench<V: Verifier>(&self, b: &Bencher);
}

fn smoke_test_insert<T: ByteMap>(t: &mut T) {
	t.insert("foo".as_bytes(), "bar".to_datum());
}

fn smoke_test_get<T: ByteMap>(t: &mut T) {
	t.insert("foo".as_bytes(), "bar".to_datum());
	assert_eq!(t.get("foo".as_bytes()).unwrap().unwrap(), "bar".as_bytes());
	assert_eq!(t.get("fooo".as_bytes()), None);
	assert_eq!(t.get("fop".as_bytes()), None);
	assert_eq!(t.get("fo".as_bytes()), None);
}

fn smoke_test_delete<T: ByteMap>(t: &mut T) {
	t.insert("foo".as_bytes(), "bar".to_datum());
	t.insert("sna".as_bytes(), "foo".to_datum());
	assert_eq!(t.get("foo".as_bytes()).unwrap().unwrap(), "bar".as_bytes());
	assert_eq!(t.get("sna".as_bytes()).unwrap().unwrap(), "foo".as_bytes());
	assert_eq!(t.get("fop".as_bytes()), None);

	t.delete("sna".as_bytes());
	assert_eq!(t.get("foo".as_bytes()).unwrap().unwrap(), "bar".as_bytes());
	assert_eq!(t.get("sna".as_bytes()), None);
	assert_eq!(t.get("fop".as_bytes()), None);
}

// The idea is that eventually, we will use these commands to simulate the DB.
// TODO: what is max key size? value size?
// enum TestCommand {
// 	GET(Value),
// 	PUT(Value, Value),
// 	DELETE(Value)
// }

// fn simple_set_put_and_get() -> impl Iterator<TestCommand> {

// }

// fn simple_set_put_get_delete() -> impl Iterator<TestCommand> {

// }

fn rng(seed: usize) -> impl Rng {
	StdRng::from_seed(&[seed])
}

fn random_byte_strings(seed: usize) -> Box<[[u8; 8]]> {
	let mut x = rng(seed);
	let mut v = Vec::<[u8; 8]>::new();

	for i in 0..1000000 {
		let rnum = x.next_u64();
		let bytes: [u8; 8] = unsafe { std::mem::transmute(rnum) };
		v.push(bytes);
	}

	v.into_boxed_slice()
}

// TODO: how does hitchhiker do benchmarks?
fn bench_put<T: ByteMap>(t: &mut T, b: &mut Bencher) {
	let ks = random_byte_strings(0);
	let vs = random_byte_strings(1);

	b.bench(u64::try_from(ks.len()).unwrap(), || {
		for i in 0..ks.len() {
			// TODO: should it be called as_datum?
			t.insert(ks[i], vs[i].to_datum())
		}
	})
}

fn bench_get<T: ByteMap>(t: &mut T, b: &mut Bencher) {

}

fn bench_del<T: ByteMap>(t: &mut T, b: &mut Bencher) {

}

// fn random_big_byte_strings() -> Box<[[u8; 1024]]> {

// }

fn bench_big_keys_put<T: ByteMap>(t: &mut T, b: &mut Bencher) {

}

fn bench_big_keys_get<T: ByteMap>(t: &mut T, b: &mut Bencher) {

}

fn bench_big_keys_del<T: ByteMap>(t: &mut T, b: &mut Bencher) {

}

fn bench_big_kv_put<T: ByteMap>(t: &mut T, b: &mut Bencher) {

}

fn bench_big_kv_get<T: ByteMap>(t: &mut T, b: &mut Bencher) {

}

fn bench_big_kv_del<T: ByteMap>(t: &mut T, b: &mut Bencher) {

}

fn bench_stress<T: ByteMap>(t: &mut T, b: &mut Bencher) {

}

// Alas, this macro is verbose, but it's the best we have
// (rust doesn't have gensym, dynamic idents, &c.)
macro_rules! deftests {
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

macro_rules! defbenches {
	{ $($testable:ty: $tr:ty => { $($name:ident, $bench:path,)* }, )* } => {
        $(
        	$(
                // #[bench]
                fn $name(b: &mut self::Bencher) {
					let mut o = <$testable as Testable>::setup();
					$bench(&mut o, b);
					o.teardown();
                }
            )*
        )*
    };
    // TODO: compare. What should a compare test do?
    // Run a random command set on two trees, compare the results.
}

deftests! {
	BTree: Tree => {
		btree_smoke_test_insert, smoke_test_insert,
		btree_smoke_test_get, smoke_test_get,
		btree_smoke_test_delete, smoke_test_delete,
	},
}

defbenches! {
	BTree: Tree => {
		btree_bench_put, bench_put,
		btree_bench_get, bench_get,
		btree_bench_del, bench_del,
		btree_bench_big_keys_put, bench_big_keys_put,
		btree_bench_big_keys_get, bench_big_keys_get,
		btree_bench_big_keys_del, bench_big_keys_del,
		btree_bench_big_kv_put, bench_big_kv_put,
		btree_bench_big_kv_get, bench_big_kv_get,
		btree_bench_big_kv_del, bench_big_kv_del,
		btree_bench_stress, bench_stress,
	},
}

// TODO: large tests, comparison tests, edge case tests.

// The plan from here: implement benchmarking. Implement serialization. (See hitchhiker tree impl)
