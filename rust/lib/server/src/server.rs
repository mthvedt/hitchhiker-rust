use std::net::SocketAddr;

use tokio_proto;

use react;
use util;

/// Starts the server, using the current thread plus (n -1) more threads. Blocks indefinitely until done!
pub fn serve(handle: &util::Handle, addr: SocketAddr) {
    tokio_proto::TcpServer::new(react::TdProto::new(handle), addr).serve(|| Ok(react::TdService));
}
