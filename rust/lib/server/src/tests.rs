use std;
use test::black_box;

use env_logger;
use tokio_proto;

use server;
use util;

lazy_static! {
    static ref SERVER_HANDLE: util::Handle = {
        util::Handle::new()
    };

    // TODO: we ideally want as few tests using _SERVER as possible.
    // In particular, logic tests should be independent of communication method.
    // (What we really want is generic fixtures and tests that can run in a multi-fixture scenario,
    // a la Spring.)
    static ref _SERVER: () = {
        env_logger::init().unwrap();
        let addr = String::from("127.0.0.1:8123").parse::<std::net::SocketAddr>().unwrap();

        SERVER_HANDLE.spawn(move || server::serve(&*SERVER_HANDLE, addr));
    };

    static ref CLIENT: () = {
        *_SERVER; // make sure server exists
        ()
    };
}

#[test]
fn server_smoke_test() {
    black_box(&*CLIENT);
}

#[test]
fn simple_request() {
    
}
