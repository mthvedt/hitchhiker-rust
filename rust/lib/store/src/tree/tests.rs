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
	fn teardown(&mut self) -> ();
}

impl Testable for BTree {
	fn setup() -> Self {
		Self::new()
	}

	fn teardown(&mut self) -> () {}
}

#[test]
fn test_insert() {
	let mut t = BTree::setup();

// TODO: use an IntoDatum trait instead
	t.insert("foo".as_bytes(), "bar".to_datum());
}
