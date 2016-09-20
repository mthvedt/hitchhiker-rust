use data::*;
use tree::btree::{Tree, BTree};

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

macro_rules! deftests {
	// TODO support benchmarks
	{ $($testable:ty: $tr:ty => { $($name:ident, $tester:path,)* }, )* } => {
        $(
        	$(
                #[test]
                fn $name() {
					let mut o = <$testable as Testable>::setup();
					$tester(&mut o);
					o.teardown();
                }
            )*
        )*
    };
}

fn smoke_test_insert<T: Tree>(t: &mut T) {
	t.insert("foo".as_bytes(), "bar".to_datum());
}

fn smoke_test_get<T: Tree>(t: &mut T) {
	t.insert("foo".as_bytes(), "bar".to_datum());
	assert_eq!(t.get("foo".as_bytes()).unwrap().unwrap(), "bar".as_bytes());
	assert_eq!(t.get("fooo".as_bytes()), None);
	assert_eq!(t.get("fop".as_bytes()), None);
	assert_eq!(t.get("fo".as_bytes()), None);
}

fn smoke_test_delete<T: Tree>(t: &mut T) {
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

deftests! {
	BTree: Tree => {
		btree_smoke_test_insert, smoke_test_insert,
		btree_smoke_test_get, smoke_test_get,
		btree_smoke_test_delete, smoke_test_delete,
	},
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

// fn benchmark_set_put() -> impl Iterator<TestCommand> {

// }

// fn benchmark_set_put_then_get() -> impl Iterator<TestCommand> {

// }

// fn benchmark_set_put_then_delete() -> impl Iterator<TestCommand> {

// }

// fn benchmark_set_big_keys() -> impl Iterator<TestCommand> {

// }

// fn benchmark_set_big_values() -> impl Iterator<TestCommand> {

// }

// TODO: large tests, comparison tests, edge case tests.

// The plan from here: implement benchmarking. Implement serialization. (See hitchhiker tree impl)
