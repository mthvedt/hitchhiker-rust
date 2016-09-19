use data::*;
use tree::btree::BTree;

/*
/// test_p for test with parameters.

macro_rules! test_p {
    ($name:ident ($param:ident: $($targets:path,)*) $body:expr) => {
        $(
            #[test]
            fn concat_idents!($name, _, ) {
             	$body
            }
        )
    }
}
*/

// Smoke tests

trait Testable {
	fn setup() -> Self;
	fn teardown(mut self) -> ();
}

impl Testable for BTree {
	fn setup() -> Self {
		Self::new()
	}

	fn teardown(self) -> () {}
}

#[test]
fn smoke_test_insert() {
	let mut t = BTree::setup();

	t.insert("foo".as_bytes(), "bar".to_datum());

	t.teardown();
}

#[test]
fn smoke_test_get() {
	let mut t = BTree::setup();

	t.insert("foo".as_bytes(), "bar".to_datum());
	assert_eq!(t.get("foo".as_bytes()).unwrap().unwrap(), "bar".as_bytes());
	assert_eq!(t.get("fooo".as_bytes()), None);
	assert_eq!(t.get("fop".as_bytes()), None);
	assert_eq!(t.get("fo".as_bytes()), None);

	t.teardown();
}

#[test]
fn smoke_test_delete() {
	let mut t = BTree::setup();

	t.insert("foo".as_bytes(), "bar".to_datum());
	t.insert("sna".as_bytes(), "foo".to_datum());
	assert_eq!(t.get("foo".as_bytes()).unwrap().unwrap(), "bar".as_bytes());
	assert_eq!(t.get("sna".as_bytes()).unwrap().unwrap(), "foo".as_bytes());
	assert_eq!(t.get("fop".as_bytes()), None);

	t.delete("sna".as_bytes());
	assert_eq!(t.get("foo".as_bytes()).unwrap().unwrap(), "bar".as_bytes());
	assert_eq!(t.get("sna".as_bytes()), None);
	assert_eq!(t.get("fop".as_bytes()), None);

	t.teardown();
}
