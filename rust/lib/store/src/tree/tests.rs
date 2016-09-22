use std;
use std::convert::TryFrom;
use std::marker::PhantomData;
use std::vec::Vec;

use data::*;
use testlib::*;
use tree::btree::*;

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

deftests! {
	BTree: Tree => {
		btree_smoke_test_insert, smoke_test_insert,
		btree_smoke_test_get, smoke_test_get,
		btree_smoke_test_delete, smoke_test_delete,
	},
}
