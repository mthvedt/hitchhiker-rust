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

impl Testable for Tree {
	fn setup() -> Self {
		Self::new()
	}

	fn teardown(&mut self) -> () {}
}

#[test]
fn test_insert() {
	let t = Tree::setup();

// TODO: use an IntoDatum trait instead
	t.insert(string_to_datum("foo"), string_to_datum("bar"));
}
