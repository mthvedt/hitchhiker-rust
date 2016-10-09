use std::borrow::Borrow;
use std::ops::Deref;

use data::*;
use super::testlib::*;
use super::btree::*;

fn test_get_str<T: ByteMap>(t: &mut T, key: &str, val: Option<&str>) {
	assert_eq!(t.get(key.as_bytes()).map(|x| x.borrow().box_copy()).as_ref().map(Deref::deref), val.map(str::as_bytes));
}

fn smoke_test_insert<T: ByteMap>(t: &mut T) {
	t.insert("foo".as_bytes(), &"bar".into_datum());
	t.check_invariants();
	t.insert("fop".as_bytes(), &"baz".into_datum());
	t.check_invariants();
}

fn smoke_test_get<T: ByteMap>(t: &mut T) {
	t.insert("foo".as_bytes(), &"bar".into_datum());
	test_get_str(t, "foo", Some("bar"));
	test_get_str(t, "foooo", None);
	test_get_str(t, "fop", None);
	test_get_str(t, "fo", None);
	test_get_str(t, "poo", None);
	t.check_invariants();

	t.insert("fop".as_bytes(), &"baz".into_datum());
	test_get_str(t, "foo", Some("bar"));
	test_get_str(t, "foooo", None);
	test_get_str(t, "fop", Some("baz"));
	test_get_str(t, "fo", None);
	test_get_str(t, "poo", None);
	t.check_invariants();
}

fn smoke_test_delete<T: ByteMap>(t: &mut T) {
	t.insert("foo".as_bytes(), &"bar".into_datum());
	t.insert("sna".as_bytes(), &"foo".into_datum());
	t.insert("fop".as_bytes(), &"baz".into_datum());
	test_get_str(t, "foo", Some("bar"));
	test_get_str(t, "sna", Some("foo"));
	test_get_str(t, "fop", Some("baz"));
	t.check_invariants();

	t.delete("sna".as_bytes());
	test_get_str(t, "foo", Some("bar"));
	test_get_str(t, "sna", None);
	test_get_str(t, "fop", Some("baz"));
	t.check_invariants();

	t.delete("fop".as_bytes());
	test_get_str(t, "foo", Some("bar"));
	test_get_str(t, "sna", None);
	test_get_str(t, "fop", None);
	t.check_invariants();
}

deftests! {
	PersistentBTree: Tree => {
		pbtree_smoke_test_insert, smoke_test_insert,
		pbtree_smoke_test_get, smoke_test_get,
		pbtree_smoke_test_delete, smoke_test_delete,
	},
}
