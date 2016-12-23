use engine::traits::*;

use super::testlib;

#[test]
fn runtime_smoke_test() {
    let f = testlib::new_factory().unwrap();
    let _cx = f.handle().new_engine().unwrap().new_context("".as_ref()).unwrap();
}

#[test]
fn json_smoke_test() {
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
