use std::collections::HashMap;

use engine::traits::*;

use super::{factory, testlib};

lazy_static! {
    // TODO: Box<[u8]> instead. or even &'static
    static ref TEST_SCRIPTS: HashMap<&'static [u8], &'static [u8]> = {
        let mut m = HashMap::new();

        // TODO: a macro
        m.insert("test/smoke.js".as_ref(), include_str!("js/test/smoke.js").as_ref());

        m
    };
}

fn test_store_factory() -> factory::Factory {
    testlib::test_store_factory(TEST_SCRIPTS.clone()).unwrap()
}

#[test]
fn runtime_smoke_test() {
    testlib::empty_store_factory().unwrap().handle().new_engine().unwrap().new_context("".as_ref()).unwrap();
}

#[test]
fn script_store_smoke_test() {
    test_store_factory().handle().new_engine().unwrap().new_context("test/smoke.js".as_ref()).unwrap();
    // let f = testlib::new_factory().unwrap();
    // let mut cx = f.handle().new_engine().unwrap().new_context().unwrap();
    // cx.exec(|acx| {
    //     acx.eval_script("test", "{}".as_ref()).unwrap();
    //     // Oddly, {} is a valid JS statement while {...object...} is not.
    //     acx.eval_script("test",
    //         r#"x = {"rpc": "2.0", "fn": "add", "callback": true, "params": [42, 23], "id": 1,
    //         "callback_to": {"site": "www.foo.bar", "port": 8888}}"#.as_ref()).unwrap();
    // });
    // // TODO test the return value
}
