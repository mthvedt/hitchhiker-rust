extern crate env_logger;
extern crate tokio_proto;

extern crate thunderhead_server;

fn main() {
    env_logger::init().unwrap();
    let addr = std::env::args().nth(1).unwrap_or("127.0.0.1:8080".to_string());
    let addr = addr.parse::<std::net::SocketAddr>().unwrap();

    tokio_proto::TcpServer::new(thunderhead_server::TdProto, addr).serve(|| Ok(thunderhead_server::TdService));
}
