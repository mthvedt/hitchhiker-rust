use std::borrow::Borrow;
use std::ops::Deref;

use data::*;
use super::testlib::*;
use super::btree::*;

fn test_data<D: Datum>(v1: Option<&D>, v2: Option<&str>) {
	// TODO: can we simplify this with API changes?
	assert_eq!(v1.map(|x| x.box_copy()).as_ref().map(Deref::deref), v2.map(str::as_bytes));
}

fn test_get_str<T: ByteMap>(t: &mut T, key: &str, val: Option<&str>) {
	// borrow checker tricks
	match t.get(key.as_bytes()) {
		Some(b) => test_data(Some(b.borrow()), val),
		None => test_data::<T::GetDatum>(None, val),
	}
}

fn smoke_test_insert<T: MutableByteMap>(t: &mut T) {
	t.insert("foo".as_bytes(), &"bar".into_datum());
	t.check_invariants();
	t.insert("fop".as_bytes(), &"baz".into_datum());
	t.check_invariants();
}

fn smoke_test_get<T: MutableByteMap>(t: &mut T) {
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

fn smoke_test_delete<T: MutableByteMap>(t: &mut T) {
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

fn smoke_test_snapshot<T: FunctionalByteMap>(t: &mut T) {
	t.insert("foo".as_bytes(), &"bar".into_datum());
	test_get_str(t, "foo", Some("bar"));
	t.check_invariants();

	let mut t0 = t.snap();
	test_get_str(t, "foo", Some("bar"));
	test_get_str(&mut t0, "foo", Some("bar"));
	t.check_invariants();
	t0.check_invariants();

	t.insert("fop".as_bytes(), &"baz".into_datum());
	test_get_str(t, "foo", Some("bar"));
	test_get_str(&mut t0, "foo", Some("bar"));
	test_get_str(t, "fop", Some("baz"));
	test_get_str(&mut t0, "fop", None);
	t.check_invariants();
	t0.check_invariants();

	// TODO: test deletion
}

fn smoke_test_diffs<T: FunctionalByteMap>(t: &mut T) {
	t.insert("foo0".as_bytes(), &"bar0".into_datum());
	let t0 = t.snap();
	t.insert("foo1".as_bytes(), &"bar1".into_datum());
	let t1 = t.snap();
	t.insert("foo2".as_bytes(), &"bar2".into_datum());
	let t2 = t.snap();
	t.insert("foo3".as_bytes(), &"bar3".into_datum());

	let c1 = t1.txid();
	let mut snap12 = t2.diff(c1);
	test_get_str(&mut snap12, "foo0", None); // TODO: failing here. Whoops, it turns out buckets need ptrs.
	test_get_str(&mut snap12, "foo1", None);
	test_get_str(&mut snap12, "foo2", Some("bar2"));
	test_get_str(&mut snap12, "foo3", None);

	let c0 = t0.txid();
	let mut snap02 = t2.diff(c0);
	test_get_str(&mut snap02, "foo0", None);
	test_get_str(&mut snap02, "foo1", Some("bar1"));
	test_get_str(&mut snap02, "foo2", Some("bar2"));
	test_get_str(&mut snap02, "foo3", None);

	let mut snap01 = t1.diff(c0);
	test_get_str(&mut snap01, "foo0", None);
	test_get_str(&mut snap01, "foo1", Some("bar1"));
	test_get_str(&mut snap01, "foo2", None);
	test_get_str(&mut snap01, "foo3", None);

	// TODO: test deletion
}

fn smoke_test_cursors<T: FunctionalByteMap>(t: &mut T) {
	t.insert("foo".as_bytes(), &"bar".into_datum());
	t.insert("sna".as_bytes(), &"foo".into_datum());
	t.insert("fop".as_bytes(), &"baz".into_datum());

	// Test beginning cursors
	let snap = t.snap();
	let mut c = snap.start_cursor();

	test_data(c.key().as_ref(), Some("foo"));
	test_data(c.value().as_ref().map(Borrow::borrow), Some("bar"));

	c.advance();
	test_data(c.key().as_ref(), Some("fop"));
	test_data(c.value().as_ref().map(Borrow::borrow), Some("baz"));

	c.advance();
	test_data(c.key().as_ref(), Some("sna"));
	test_data(c.value().as_ref().map(Borrow::borrow), Some("foo"));

	c.advance();
	test_data(c.key().as_ref(), None);
	test_data(c.value().as_ref().map(Borrow::borrow), None);
	// Does modifying the tree affect the cursor?

	// Test middle cursors

	// TODO: test deletion
}

fn smoke_test_diff_cursors<T: FunctionalByteMap>(t: &mut T) {
	// TODO
}

// TODO: maybe these should just be normal tests? are we going with only one type of tree or multiple?
deftests! {
	PersistentBTree => {
		pbtree_smoke_test_insert, smoke_test_insert,
		pbtree_smoke_test_get, smoke_test_get,
		// pbtree_smoke_test_delete, smoke_test_delete,
		pbtree_smoke_test_snapshot, smoke_test_snapshot,
		pbtree_smoke_test_diffs, smoke_test_diffs,
		pbtree_smoke_test_cursors, smoke_test_cursors,
		pbtree_smoke_test_diff_cursors, smoke_test_diff_cursors,
	},
}
