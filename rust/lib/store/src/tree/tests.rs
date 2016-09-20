// TODO: WTF do we need 'tree::'?
use tree::test::Bencher;

use data::*;
use tree::btree::{ByteMap, ByteTree, BTree};

trait Testable {
	fn setup() -> Self;
	fn teardown(mut self) -> ();
}

impl Testable for BTree {
	fn setup() -> Self {
		Self::new()
	}

	fn teardown(self) {}
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

// fn random_byte_strings() -> Box<[[u8; 8]]> {

// }

// TODO: how does hitchhiker do benchmarks?
fn bench_put<T: ByteMap>(t: &T, b: &mut Bencher) {

}

fn bench_get<T: ByteMap>(t: &T, b: &mut Bencher) {

}

fn bench_del<T: ByteMap>(t: &T, b: &mut Bencher) {

}

// fn random_big_byte_strings() -> Box<[[u8; 1024]]> {

// }

fn bench_big_keys_put<T: ByteMap>(t: &T, b: &mut Bencher) {

}

fn bench_big_keys_get<T: ByteMap>(t: &T, b: &mut Bencher) {

}

fn bench_big_keys_del<T: ByteMap>(t: &T, b: &mut Bencher) {

}

fn bench_big_kv_put<T: ByteMap>(t: &T, b: &mut Bencher) {

}

fn bench_big_kv_get<T: ByteMap>(t: &T, b: &mut Bencher) {

}

fn bench_big_kv_del<T: ByteMap>(t: &T, b: &mut Bencher) {

}

fn bench_stress<T: ByteMap>(t: &T, b: &mut Bencher) {

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
                #[bench]
                fn $name(b: &mut Bencher) {
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
