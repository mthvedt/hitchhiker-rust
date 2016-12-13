use engine::traits::*;

use super::Spec;

#[test]
fn runtime_smoke_test() {
    let f = Spec::new_factory().unwrap();
    let _cx = f.handle().new_engine().unwrap().new_context().unwrap();
}

#[test]
fn json_smoke_test() {
    let f = Spec::new_factory().unwrap();
    let mut cx = f.handle().new_engine().unwrap().new_context().unwrap();
    // cx.parse_json("{}".as_ref()).ok().unwrap();
    //
    // cx.parse_json(r#"{"rpc": "2.0", "fn": "add", "callback": true, "params": [42, 23], "id": 1,
    // "callback_to": {"site": "www.foo.bar", "port": 8888}}"#.as_ref()).ok().unwrap();
    // TODO test result
}
