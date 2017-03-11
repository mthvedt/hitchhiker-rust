#![feature(try_from)]
#![feature(test)]

extern crate rand;
extern crate test;

#[macro_use]
extern crate htree;

use std::borrow::Borrow;
use std::collections::*;
use std::convert::TryFrom;
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::ops::Deref;

use test::black_box;

use rand::{Rng, SeedableRng};

use htree::*;

use htree::bench::*;
use htree::testlib::*;

// TODO delete these traits
use htree::data::Key;
use htree::data::Datum;
use htree::data::IntoDatum;
use htree::data::RcBytes;

fn insert_hashmap<V: Verifier>(m: &mut HashMap<Vec<u8>, Vec<u8>>, k: &[u8], v: &[u8]) {
	V::run(|| m.insert(Vec::from_iter(k.iter().cloned()), Vec::from_iter(v.iter().cloned())));
}

fn check_hashmap<T: ByteMap, V: Verifier>(t: &mut T, m: &HashMap<Vec<u8>, Vec<u8>>, k: &[u8]) {
	V::verify(|| "map get mismatch",
		|| m.get(k).map(|x| x.as_ref())
		   == t.get(k).map(|x| x.borrow().box_copy()).as_ref().map(Box::deref));
}

// TODO rename
// fn delete_hashmap<V: Verifier>(m: &mut HashMap<Vec<u8>, Vec<u8>>, k: &[u8]) {
// 	V::run(|| m.remove(&Vec::from_iter(k.iter().cloned())));
// }

defbench! {
	// This serves as a smoke test--it should give the same benchmarks as bench_put below.
	bench_ref_std_map, _t: Testable, b, T, V, {
		// TODO: use the same rand everywhere.
		let ks = random_byte_strings(0xC400D969);
		let vs = random_byte_strings(0x3FB87EE6);
		let kslices = ks.iter().map(|x| x.as_ref()).collect::<Vec<&[u8]>>();
		let vslices = vs.iter().map(|x| x.as_ref()).collect::<Vec<&[u8]>>();
		let mut m = HashMap::new();

		b.bench(u64::try_from(ks.len()).unwrap(), || {
			for i in 0..ks.len() {
				m.insert(kslices[i], vslices[i]);
			}
		})
	}
}

defbench! {
	// This serves as a smoke test--it should give the same benchmarks as bench_put below.
	bench_put_no_verify, t: MutableByteMap, b, T, V, {
		// Note that the seeds are the same as bench_put. This is on purpose.
		let ks = random_byte_strings(0xC400D969);
		let vs = random_byte_strings(0x3FB87EE6);

		b.bench(u64::try_from(ks.len()).unwrap(), || {
			for i in 0..ks.len() {
				t.insert(&ks[i], &vs[i].into_datum())
			}
		})
	}
}

// TODO: how does hitchhiker do benchmarks?
// TODO: we can parameterize these better... we want a test with 100 byte keys, and 100 byte key 8k value.
// We also want to record bytes/sec.
defbench! {
	bench_put, t: MutableByteMap, b, T, V, {
		let ks = random_byte_strings(0xBCA2E7D6);
		let vs = random_byte_strings(0xA8541B4F);
		// TODO: mix some valid k's into here.
		let rand_tests = random_byte_strings(0x0BACE2CE);

		// TODO why use a map when we can test directly? We should force ourselves to write out the expected
		// results anyway.
		let mut m = HashMap::new();

		b.bench(u64::try_from(ks.len()).unwrap(), || {
			for i in 0..ks.len() {
				t.insert(&ks[i], &vs[i].into_datum());
				insert_hashmap::<V>(&mut m, &ks[i], &vs[i]);
				check_hashmap::<T, V>(t, &m, &ks[i]);
				check_hashmap::<T, V>(t, &m, &rand_tests[i]);
			}
		})
	}
}

defbench! {
	bench_get, t: MutableByteMap, b, T, V, {
		let ks = random_byte_strings(0x45421572);
		let vs = random_byte_strings(0x80E9F4A6);
		let rand_tests = random_byte_strings(0xE06759F4);

		let mut m = HashMap::new();

		for i in 0..ks.len() {
			t.insert(&ks[i], &vs[i].into_datum());
			insert_hashmap::<V>(&mut m, &ks[i], &vs[i]);
		}

		b.bench(u64::try_from(ks.len()).unwrap(), || {
			for i in 0..ks.len() {
				black_box(t.get(&ks[i]));
				check_hashmap::<T, V>(t, &m, &ks[i]);
				check_hashmap::<T, V>(t, &m, &rand_tests[i]);
			}
		})
	}
}

// defbench! {
// 	bench_del, t: ByteMap, b, T, V, {
// 		let ks = random_byte_strings(0xC400D969);
// 		let vs = random_byte_strings(0x3FB87EE6);
// 		let rand_tests = random_byte_strings(0x6E7D2E0F);

// 		let mut m = HashMap::new();

// 		for i in 0..ks.len() {
// 			t.insert(&ks[i], &vs[i].into_datum());
// 			insert_hashmap::<V>(&mut m, &ks[i], &vs[i]);
// 		}

// 		b.bench(u64::try_from(ks.len()).unwrap(), || {
// 			for i in 0..ks.len() {
// 				black_box(t.delete(&ks[i]));
// 				delete_hashmap::<V>(&mut m, &ks[i]);
// 				check_hashmap::<T, V>(t, &m, &ks[i]);
// 				check_hashmap::<T, V>(t, &m, &rand_tests[i]);
// 			}
// 		})
// 	}
// }

// defbench! {
// 	bench_put_big, t: ByteMap, b, T, V, {
// 		let ks = random_big_byte_strings(0xC400D969);
// 		let vs = random_big_byte_strings(0x3FB87EE6);
// 		let rand_tests = random_big_byte_strings(0x6E7D2E0F);

// 		let mut m = HashMap::new();

// 		b.bench(u64::try_from(ks.len()).unwrap(), || {
// 			for i in 0..ks.len() {
// 				t.insert(&ks[i], &vs[i].into_datum());
// 				insert_hashmap::<V>(&mut m, &ks[i], &vs[i]);
// 				check_hashmap::<T, V>(t, &m, &ks[i]);
// 				check_hashmap::<T, V>(t, &m, &rand_tests[i]);
// 			}
// 		})
// 	}
// }

// defbench! {
// 	bench_get_big, t: ByteMap, b, T, V, {
// 		let ks = random_big_byte_strings(0xC400D969);
// 		let vs = random_big_byte_strings(0x3FB87EE6);
// 		let rand_tests = random_big_byte_strings(0x6E7D2E0F);

// 		let mut m = HashMap::new();

// 		for i in 0..ks.len() {
// 			t.insert(&ks[i], &vs[i].into_datum());
// 			insert_hashmap::<V>(&mut m, &ks[i], &vs[i]);
// 		}

// 		b.bench(u64::try_from(ks.len()).unwrap(), || {
// 			for i in 0..ks.len() {
// 				black_box(t.get(&ks[i]));
// 				check_hashmap::<T, V>(t, &m, &ks[i]);
// 				check_hashmap::<T, V>(t, &m, &rand_tests[i]);
// 			}
// 		})
// 	}
// }

// defbench! {
// 	bench_del_big, t: ByteMap, b, T, V, {
// 		let ks = random_big_byte_strings(0xC400D969);
// 		let vs = random_big_byte_strings(0x3FB87EE6);
// 		let rand_tests = random_big_byte_strings(0x6E7D2E0F);

// 		let mut m = HashMap::new();

// 		for i in 0..ks.len() {
// 			t.insert(&ks[i], &vs[i].into_datum());
// 			insert_hashmap::<V>(&mut m, &ks[i], &vs[i]);
// 		}

// 		b.bench(u64::try_from(ks.len()).unwrap(), || {
// 			for i in 0..ks.len() {
// 				black_box(t.delete(&ks[i]));
// 				delete_hashmap::<V>(&mut m, &ks[i]);
// 				check_hashmap::<T, V>(t, &m, &ks[i]);
// 				check_hashmap::<T, V>(t, &m, &rand_tests[i]);
// 			}
// 		})
// 	}
// }

// defbench! {
// 	bench_put_huge_values, t: ByteMap, b, T, V, {
// 		let ks = random_big_byte_strings(0xC400D969);
// 		let vs = random_huge_byte_strings(0x3FB87EE6);
// 		let rand_tests = random_big_byte_strings(0x6E7D2E0F);

// 		let mut m = HashMap::new();

// 		b.bench(u64::try_from(ks.len()).unwrap(), || {
// 			for i in 0..ks.len() {
// 				t.insert(&ks[i], &vs[i].into_datum());
// 				insert_hashmap::<V>(&mut m, &ks[i], &vs[i]);
// 				check_hashmap::<T, V>(t, &m, &ks[i]);
// 				check_hashmap::<T, V>(t, &m, &rand_tests[i]);
// 			}
// 		})
// 	}
// }

// defbench! {
// 	bench_get_huge_values, t: ByteMap, b, T, V, {
// 		let ks = random_big_byte_strings(0xC400D969);
// 		let vs = random_huge_byte_strings(0x3FB87EE6);
// 		let rand_tests = random_big_byte_strings(0x6E7D2E0F);

// 		let mut m = HashMap::new();

// 		for i in 0..ks.len() {
// 			t.insert(&ks[i], &vs[i].into_datum());
// 			insert_hashmap::<V>(&mut m, &ks[i], &vs[i]);
// 		}

// 		b.bench(u64::try_from(ks.len()).unwrap(), || {
// 			for i in 0..ks.len() {
// 				black_box(t.get(&ks[i]));
// 				check_hashmap::<T, V>(t, &m, &ks[i]);
// 				check_hashmap::<T, V>(t, &m, &rand_tests[i]);
// 			}
// 		})
// 	}
// }

// defbench! {
// 	bench_del_huge_values, t: ByteMap, b, T, V, {
// 		let ks = random_big_byte_strings(0xC400D969);
// 		let vs = random_huge_byte_strings(0x3FB87EE6);
// 		let rand_tests = random_huge_byte_strings(0x6E7D2E0F);

// 		let mut m = HashMap::new();

// 		for i in 0..ks.len() {
// 			t.insert(&ks[i], &vs[i].into_datum());
// 			insert_hashmap::<V>(&mut m, &ks[i], &vs[i]);
// 		}

// 		b.bench(u64::try_from(ks.len()).unwrap(), || {
// 			for i in 0..ks.len() {
// 				black_box(t.delete(&ks[i]));
// 				delete_hashmap::<V>(&mut m, &ks[i]);
// 				check_hashmap::<T, V>(t, &m, &ks[i]);
// 				check_hashmap::<T, V>(t, &m, &rand_tests[i]);
// 			}
// 		})
// 	}
// }

// defbench! {
// 	bench_put_huge, t: ByteMap, b, T, V, {
// 		let ks = random_huge_byte_strings(0xC400D969);
// 		let vs = random_huge_byte_strings(0x3FB87EE6);
// 		let rand_tests = random_huge_byte_strings(0x6E7D2E0F);

// 		let mut m = HashMap::new();

// 		for i in 0..ks.len() {
// 			t.insert(&ks[i], &vs[i].into_datum());
// 			insert_hashmap::<V>(&mut m, &ks[i], &vs[i]);
// 		}

// 		b.bench(u64::try_from(ks.len()).unwrap(), || {
// 			for i in 0..ks.len() {
// 				t.insert(&ks[i], &vs[i].into_datum());
// 				insert_hashmap::<V>(&mut m, &ks[i], &vs[i]);
// 				check_hashmap::<T, V>(t, &m, &ks[i]);
// 				check_hashmap::<T, V>(t, &m, &rand_tests[i]);
// 			}
// 		})
// 	}
// }

// defbench! {
// 	bench_get_huge, t: ByteMap, b, T, V, {
// 		let ks = random_huge_byte_strings(0xC400D969);
// 		let vs = random_huge_byte_strings(0x3FB87EE6);
// 		let rand_tests = random_huge_byte_strings(0x6E7D2E0F);

// 		let mut m = HashMap::new();

// 		for i in 0..ks.len() {
// 			t.insert(&ks[i], &vs[i].into_datum());
// 			insert_hashmap::<V>(&mut m, &ks[i], &vs[i]);
// 		}

// 		b.bench(u64::try_from(ks.len()).unwrap(), || {
// 			for i in 0..ks.len() {
// 				black_box(t.get(&ks[i]));
// 				check_hashmap::<T, V>(t, &m, &ks[i]);
// 				check_hashmap::<T, V>(t, &m, &rand_tests[i]);
// 			}
// 		})
// 	}
// }

// defbench! {
// 	bench_del_huge, t: ByteMap, b, T, V, {
// 		let ks = random_huge_byte_strings(0xC400D969);
// 		let vs = random_huge_byte_strings(0x3FB87EE6);
// 		let rand_tests = random_huge_byte_strings(0x6E7D2E0F);

// 		let mut m = HashMap::new();

// 		for i in 0..ks.len() {
// 			t.insert(&ks[i], &vs[i].into_datum());
// 			insert_hashmap::<V>(&mut m, &ks[i], &vs[i]);
// 		}

// 		b.bench(u64::try_from(ks.len()).unwrap(), || {
// 			for i in 0..ks.len() {
// 				black_box(t.delete(&ks[i]));
// 				delete_hashmap::<V>(&mut m, &ks[i]);
// 				check_hashmap::<T, V>(t, &m, &ks[i]);
// 				check_hashmap::<T, V>(t, &m, &rand_tests[i]);
// 			}
// 		})
// 	}
// }

// fn bench_stress<T: ByteMap>(t: &mut T, b: &mut Bencher) {

// }

// Benchmark that makes one snapshot each time something is inserted.
defbench! {
	bench_snapshots_frequent, t: FunctionalByteMap, b, T, V, {
		let mut r = b.rand();

		let ks = random_byte_strings(0xBCA2E7D6);
		let vs = random_byte_strings(0xA8541B4F);
		let mut snapvec = Vec::with_capacity(ks.len());

		b.bench(u64::try_from(ks.len()).unwrap(), || {
			for i in 0..ks.len() {
				t.insert(&ks[i], &vs[i].into_datum());
				// Snap number i contains keys 1..i
				snapvec.push(t.snap());

				// Random verification
				V::verify(|| "map get mismatch", || {
					let idx = r.next_u32() as usize % ks.len();
					if idx <= i {
						t.get(&ks[idx]).map_or(false, |r| r.borrow().box_copy().deref() == &vs[idx])
					} else {
						t.get(&ks[idx]).is_none()
					}
				});

				V::verify(|| "snapshot mismatch", || {
					let idx = r.next_u32() as usize % ks.len();
					let snapid = r.next_u32() as usize % snapvec.len();
					if idx <= snapid {
						snapvec[snapid].get(&ks[idx]).map_or(false, |r| r.borrow().box_copy().deref() == &vs[idx])
					} else {
						snapvec[snapid].get(&ks[idx]).is_none()
					}
				});
			}
		})
	}
}

// TODO: why use macros anyway?

// Benchmark that calls get on random diff snapshots. Approximately 3/4 of gets are misses.
defbench! {
	bench_diff_get, t: FunctionalByteMap, b, T, V, {
		let mut r = b.rand();

		let ks = random_byte_strings(0xBCA2E7D6);
		let vs = random_byte_strings(0xA8541B4F);
		let mut snapvec = Vec::with_capacity(ks.len());
		let mut random_diff_counters = Vec::with_capacity(ks.len());
		let mut diffvec = Vec::with_capacity(ks.len());
		let mut random_get_indices = Vec::with_capacity(ks.len());
		let mut random_get_keys = Vec::with_capacity(ks.len());

		for i in 0..ks.len() {
			t.insert(&ks[i], &vs[i].into_datum());
			// Snap number i contains keys 0..i inclusive
			let snap = t.snap();
			// Push snap first to help type inference.
			snapvec.push(snap);

			// randomly choose a tx to make a diff snapshot
			let random_diff_counter = snapvec[r.next_u32() as usize % (i + 1)].txid();
			let diff = snapvec[snapvec.len() - 1].diff(random_diff_counter);

			random_diff_counters.push(random_diff_counter);
			diffvec.push(diff);

			// Make the random get index a key that was committed at or before the current diff.
			// It has about a 1/2 chance of showing up in the actual diff.
			let random_get_index = r.next_u32() as usize % ks.len();
			random_get_indices.push(random_get_index);
			random_get_keys.push(&ks[random_get_index]);
		}

		b.bench(u64::try_from(ks.len()).unwrap(), || {
			for i in 0..ks.len() {
				black_box(diffvec[i].get(random_get_keys[i]));

				V::verify(|| "diff mismatch", || {
					let random_get_index = random_get_indices[i];
					let r = diffvec[i].get(random_get_keys[i]);

					// Slightly tricky.
					// Our snapshot diff is bounded by snapshots (x, y) where x is the snapshot
					// with counter random_diff_counters[i], and y is the snapshot at snap[i]

					// println!("{} {} {} {}", i, random_get_index, random_diff_counters[i], snapvec[random_get_index].txid());
					if random_get_index > i {
						// Is out of the maximum bound of our snapshot
						r.is_none()
					} else if snapvec[random_get_index].txid().circle_lt_eq(random_diff_counters[i]) {
						// a little confusing. the snapshot at random_get_index is the first snapshot that contains the key
						// at random_get_index. if the diff doesn't contain that snapshot's txid, it shouldn't contain that key.
						r.is_none()
					} else {
						r.map_or(false, |r| r.borrow().box_copy().deref() == &vs[random_get_index])
					}
				});
			}
		})
	}
}

defbench! {
	bench_cursor_scan, t: FunctionalByteMap, b, T, V, {
		let ks = random_byte_strings(0xBCA2E7D6);
		let vs = random_byte_strings(0xA8541B4F);
		let mut m = HashMap::new();

		for i in 0..ks.len() {
			t.insert(&ks[i], &vs[i]);
			m.insert(ks[i].as_ref(), vs[i].as_ref());
		}

		let snap = t.snap();
		let mut cursor = snap.start_cursor();
		let mut i = 0;
		let mut most_recent_key: Option<RcBytes> = None;

		b.bench(u64::try_from(ks.len()).unwrap(), || {
			loop {
				if cursor.key().is_none() {
					break;
				}

				black_box(&cursor.key().unwrap());
				black_box(&cursor.value().unwrap());

				V::verify(|| "n/a", || {
					i += 1;
					true
				});
				V::verify(|| "cursor out of order", || {
					let k = cursor.key();
					let r = match most_recent_key {
						Some(ref k2) => k.as_ref().unwrap().bytes() >= k2.bytes(),
						None => true,
					};
					most_recent_key = k;
					r
				});
				V::verify(|| "cursor mismatch", || {
					m.get(cursor.key().unwrap().bytes()).unwrap() ==
					&cursor.value().unwrap().borrow().box_copy().deref()
				});

				cursor.advance();
			}
		});

		V::verify(|| "wrong number of items in cursor", || i == ks.len())
	}
}

fn main() {
	// TODO: use cargo to default to release, but enable both modes
	// debug_assert!(false, "This target should be run in release mode");

	let benchmarks = create_benchmarks! {
		[
			PersistentBTree,
		] => [
			bench_snapshots_frequent,
			bench_cursor_scan,
			bench_diff_get,
		],
		[DummyTestable,] => [bench_ref_std_map,],
		[
			ByteHashMap,
			ByteTreeMap,
			PersistentBTree,
		] => [
			// bench_put_no_verify,
		    bench_put,
		    bench_get,
			// bench_del,
			// bench_put_big,
			// bench_get_big,
			// bench_del_big,
		],
		// [
		// 	ByteHashMap,
		// 	ByteTreeMap,
		// 	BTree,
		// ] => [
		// 	bench_put_no_verify,
		// 	bench_put,
		// 	bench_get,
		// 	bench_del,
		// 	bench_put_big,
		// 	bench_get_big,
		// 	bench_del_big,
		// 	// bench_put_huge_values,
		// 	// bench_get_huge_values,
		// 	// bench_del_huge_values,
		// 	// bench_put_huge,
		// 	// bench_get_huge,
		// 	// bench_del_huge,
		// 	// bench_stress,
		// ],
	};

	run_benchmarks(&benchmarks, &mut std::io::stdout());
}
