// use super::RuntimeHandle;
//
// #[test]
// fn runtime_smoke_test() {
//     RuntimeHandle::new_runtime().new_environment().context();
// }
//
// #[test]
// fn json_smoke_test() {
//     let mut r = RuntimeHandle::new_runtime();
//     let mut env = r.new_environment();
//     let mut cx = env.context();
//     cx.parse_json("{}".as_ref()).ok().unwrap();
//
//     cx.parse_json(r#"{"rpc": "2.0", "fn": "add", "callback": true, "params": [42, 23], "id": 1,
//     "callback_to": {"site": "www.foo.bar", "port": 8888}}"#.as_ref()).ok().unwrap();
//     // TODO test result
// }
