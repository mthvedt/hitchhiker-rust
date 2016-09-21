extern crate rand;
extern crate test;
extern crate time;

use std;
use std::convert::TryFrom;
use std::marker::PhantomData;
use std::vec::Vec;

use self::rand::*;
use self::time::*;
use self::test::black_box;

use data::*;
use tree::btree::{ByteMap, ByteTree, BTree};

trait Testable {
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

enum BenchResult {
	Ok(Duration, u64),
	Fail(String),
	None,
}

struct Bencher {
	result: BenchResult,
}

impl Bencher {
	fn new() -> Bencher {
		Bencher {
			result: BenchResult::None,
		}
	}

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
	fn verify<F, MF>(message: MF, f: F) where F: FnOnce() -> bool, MF: FnOnce() -> String;
	fn verify_custom<F>(f: F) where F: FnOnce() -> Option<String>;
}

/// A verifier that does nothing. If
struct NullVerifier {}

impl Verifier for NullVerifier {
	fn run_update<F>(f: F) where F: FnOnce() {}
	fn verify<F, MF>(message: MF, f: F) where F: FnOnce() -> bool, MF: FnOnce() -> String {}
	fn verify_custom<F>(f: F) where F: FnOnce() -> Option<String> {}
}

struct RealVerifier {}

impl Verifier for RealVerifier {
	fn run_update<F>(f: F) where F: FnOnce() { f() }

	fn verify<F, MF>(message: MF, f: F) where F: FnOnce() -> bool, MF: FnOnce() -> String {
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

/// A benchmarkable closure of some kind, post-parameterization.
// TODO: rename.
trait Benchable {
	fn name(&self) -> (String, String);
	fn bench(&self, b: &mut Bencher);
	fn verify(&self);
}

macro_rules! defbench {
	{ $name:ident, $id1:ident: $id1trait:ident, $idbencher:ident, $idverifier:ident, $e:expr } => {
		// Basically, we want to 'bundle' Testables into Benchables, parameterized by different kinds of Testable.
		// This ugly macro is the easiest way to do it: we get a nice bundle of Benchable at the end, parameterizable by
		// a statically-checked type. If we had HKTs or typeclasses like Haskell,
		// we could imagine a more elegant way using fn composition... but we have neither of those.

		fn $name<_T: $id1trait + Testable + 'static>() -> Box<Benchable> {
			// We have to have this local type to get around a limitation: rustc can't capture _T.
			struct _voldemort_benchable<_Tcap: $id1trait + Testable + 'static> {
				_phantom: PhantomData<_Tcap>,
			}

			impl<_Tcap: $id1trait + Testable + 'static> _voldemort_benchable<_Tcap> {
				fn _voldemort_bench<$idverifier: Verifier>(&self, $id1: &mut _Tcap, $idbencher: &mut Bencher) {
					$e
				}
			}

			impl<_Tcap: $id1trait + Testable + 'static> Benchable for _voldemort_benchable<_Tcap> {
				fn name(&self) -> (String, String) { (String::from(stringify!($name)), <_Tcap as Testable>::name()) }

				fn bench(&self, b: &mut Bencher) {
					let mut t = _Tcap::setup();
					self._voldemort_bench::<NullVerifier>(&mut t, b);
					t.teardown();
				}

				fn verify(&self) {
					// Unused bencher
					let mut t = _Tcap::setup();
					let mut b = Bencher::new();
					self._voldemort_bench::<RealVerifier>(&mut t, &mut b);
					t.teardown();
				}
			}

			Box::new(_voldemort_benchable::<_T> {
				_phantom: PhantomData,
			})
		}
	};
}

// TODO: how does hitchhiker do benchmarks?
defbench! {
	bench_put, t: ByteMap, b, V, {
		let ks = random_byte_strings(0);
		let vs = random_byte_strings(1);

		b.bench(u64::try_from(ks.len()).unwrap(), || {
			for i in 0..ks.len() {
				// TODO: should it be called as_datum?
				t.insert(ks[i], vs[i].to_datum())
			}
		})
	}
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

macro_rules! _create_benchmarks_helper {
	{ $vec:ident, $testable:ty, [ $($benchf:ident,)* ] } => {
		$(
	        $vec.push($benchf::<$testable>());
	    )*
    };
}

/// Create a vec of boxed deparameterized benchmarks, to be run at one's leisure.
macro_rules! create_benchmarks {
	{ $([ $($testable:ty,)* ] => $benchf_list:tt,)* } => {
		{
			let mut _r: Vec<Box<Benchable>> = Vec::new();

			$($(
				_create_benchmarks_helper! {_r, $testable, $benchf_list}
	        )*)*

	        _r
    	}
    };
}

deftests! {
	BTree: Tree => {
		btree_smoke_test_insert, smoke_test_insert,
		btree_smoke_test_get, smoke_test_get,
		btree_smoke_test_delete, smoke_test_delete,
	},
}

// trace_macros!(true);
fn benchmark_suite() -> Vec<Box<Benchable>> {
	create_benchmarks! {
		[BTree,] => [
			bench_put,
			// bench_get,
			// bench_del,
			// bench_big_keys_put,
			// bench_big_keys_get,
			// bench_big_keys_del,
			// bench_big_kv_put,
			// bench_big_kv_get,
			// bench_big_kv_del,
			// bench_stress,
		],
	}
}
// trace_macros!(false)

// TODO: large tests, comparison tests, edge case tests.

// The plan from here: implement benchmarking. Implement serialization. (See hitchhiker tree impl)
