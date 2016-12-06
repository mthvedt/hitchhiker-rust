use std;

use env_logger;
use tokio_proto;

use server;
use util;

lazy_static! {
    static ref SERVER_HANDLE: util::Handle = {
        util::Handle::new()
    };

    static ref _SERVER: () = {
        env_logger::init().unwrap();
        let addr = std::env::args().nth(1).unwrap_or("127.0.0.1:8080".to_string());
        let addr = addr.parse::<std::net::SocketAddr>().unwrap();

        SERVER_HANDLE.spawn(move || server::serve(&*SERVER_HANDLE, addr));
    };
}
