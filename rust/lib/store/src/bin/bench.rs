#![feature(try_from)]
#![feature(test)]

#[macro_use]
extern crate thunderhead_store;
extern crate test;

use std::convert::TryFrom;
use std::marker::PhantomData;

use thunderhead_store::*;
use thunderhead_store::bench::*;
use thunderhead_store::tree::btree::*;
use thunderhead_store::tree::testlib::*;

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

fn main() {
	// TODO: use cargo to default to release, but enable both modes
	debug_assert!(false, "This target should be run in release mode");

	let benchmarks = create_benchmarks! {
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
	};

	run_benchmarks(&benchmarks, &mut std::io::stdout());
}
