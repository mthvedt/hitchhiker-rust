#![feature(try_from)]
#![feature(test)]

#[macro_use]
extern crate thunderhead_store;
extern crate test;

use std::collections::*;
use std::convert::TryFrom;
use std::marker::PhantomData;
use std::ops::Deref;
use test::black_box;

use thunderhead_store::*;
use thunderhead_store::bench::*;
// TODO make it data::slice
// use thunderhead_store::slice::*;
use thunderhead_store::tree::btree::*;
use thunderhead_store::tree::testlib::*;

defbench! {
	// This serves as a smoke test--it should give the same benchmarks as bench_put below.
	bench_ref_std_map, _t: Testable, b, V, {
		// Note that the seeds are the same as bench_put. This is on purpose.
		let ks = random_byte_strings(0xC400D969);
		let vs = random_byte_strings(0x3FB87EE6);
		let kslices = ks.iter().map(|x| x.as_ref()).collect::<Vec<&[u8]>>();
		let vslices = vs.iter().map(|x| x.as_ref()).collect::<Vec<&[u8]>>();
		let mut m = HashMap::new();

		b.bench(u64::try_from(ks.len()).unwrap(), || {
			for i in 0..ks.len() {
				// TODO: should it be called as_datum?
				m.insert(kslices[i], vslices[i]);
			}
		})
	}
}

// TODO: how does hitchhiker do benchmarks?
defbench! {
	// This serves as a smoke test--it should give the same benchmarks as bench_put below.
	bench_put_no_verify, t: ByteMap, b, V, {
		// Note that the seeds are the same as bench_put. This is on purpose.
		let ks = random_byte_strings(0xC400D969);
		let vs = random_byte_strings(0x3FB87EE6);

		b.bench(u64::try_from(ks.len()).unwrap(), || {
			for i in 0..ks.len() {
				// TODO: should it be called as_datum?
				t.insert(&ks[i], &vs[i].into_datum())
			}
		})
	}
}

defbench! {
	bench_put, t: ByteMap, b, V, {
		let ks = random_byte_strings(0xC400D969);
		let vs = random_byte_strings(0x3FB87EE6);
		let rand_tests = random_byte_strings(0x6E7D2E0F);

		let mut m = HashMap::new();

		b.bench(u64::try_from(ks.len()).unwrap(), || {
			for i in 0..ks.len() {
				// TODO: should it be called as_datum?
				t.insert(&ks[i], &vs[i].into_datum());
				V::run(|| m.insert(ks[i], vs[i]));
				V::verify(|| "map get mismatch",
					|| m.get(&ks[i]).map(|x| x.as_ref())
					   == t.get(&ks[i]).map(Datum::box_copy).as_ref().map(Box::deref));
				V::verify(|| "map get mismatch",
					|| m.get(&rand_tests[i]).map(|x| x.as_ref())
					   == t.get(&rand_tests[i]).map(Datum::box_copy).as_ref().map(Box::deref));
			}
		})
	}
}

defbench! {
	bench_get, t: ByteMap, b, V, {
		let ks = random_byte_strings(0xC400D969);
		let vs = random_byte_strings(0x3FB87EE6);
		let rand_tests = random_byte_strings(0x6E7D2E0F);

		let mut m = HashMap::new();

		for i in 0..ks.len() {
			t.insert(&ks[i], &vs[i].into_datum());
			V::run(|| m.insert(ks[i], vs[i]));
		}

		b.bench(u64::try_from(ks.len()).unwrap(), || {
			for i in 0..ks.len() {
				// TODO: should it be called as_datum?
				black_box(t.get(&ks[i]));
				V::verify(|| "map get mismatch",
					|| m.get(&ks[i]).map(|x| x.as_ref())
					   == t.get(&ks[i]).map(Datum::box_copy).as_ref().map(Box::deref));
				V::verify(|| "map get mismatch",
					|| m.get(&rand_tests[i]).map(|x| x.as_ref())
					   == t.get(&rand_tests[i]).map(Datum::box_copy).as_ref().map(Box::deref));
			}
		})
	}
}

// fn bench_del<T: ByteMap>(t: &mut T, b: &mut Bencher) {

// }

// fn random_big_byte_strings() -> Box<[[u8; 1024]]> {

// }

// fn bench_big_keys_put<T: ByteMap>(t: &mut T, b: &mut Bencher) {

// }

// fn bench_big_keys_get<T: ByteMap>(t: &mut T, b: &mut Bencher) {

// }

// fn bench_big_keys_del<T: ByteMap>(t: &mut T, b: &mut Bencher) {

// }

// fn bench_big_kv_put<T: ByteMap>(t: &mut T, b: &mut Bencher) {

// }

// fn bench_big_kv_get<T: ByteMap>(t: &mut T, b: &mut Bencher) {

// }

// fn bench_big_kv_del<T: ByteMap>(t: &mut T, b: &mut Bencher) {

// }

// fn bench_stress<T: ByteMap>(t: &mut T, b: &mut Bencher) {

// }

fn main() {
	// TODO: use cargo to default to release, but enable both modes
	debug_assert!(false, "This target should be run in release mode");

	let benchmarks = create_benchmarks! {
		[DummyTestable,] => [bench_ref_std_map,],
		[ByteHashMap, ByteTreeMap, BTree,] => [
			bench_put_no_verify,
			bench_put,
			bench_get,
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
