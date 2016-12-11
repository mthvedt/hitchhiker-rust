extern crate env_logger;
extern crate tokio_proto;

extern crate thunderhead_server;

fn main() {
    env_logger::init().unwrap();
    let addr = "127.0.0.1:8080".into().parse::<std::net::SocketAddr>().unwrap();

    tokio_proto::TcpServer::new(thunderhead_server::TdProto, addr).serve(|| Ok(thunderhead_server::TdService));
}
