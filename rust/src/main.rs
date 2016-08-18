extern crate env_logger;
extern crate futures;
extern crate futures_minihttp;
extern crate time;
extern crate url;

use std::net::SocketAddr;
use std::env;

// TODO: create facades for our http request/response library.
use futures::*;
use futures_minihttp::{Server, Response, Request};
use url::*;

fn main() {
    env_logger::init().unwrap();
    let addr = env::args().nth(1).unwrap_or("127.0.0.1:8080".to_string());
    let addr = addr.parse::<SocketAddr>().unwrap();
    Server::new(&addr).serve(|req: Request| {
        let root_url = Url::parse("http://127.0.0.1:8080").unwrap();
        let parse_options = Url::options().base_url(Some(&root_url));
        let url = parse_options.parse(req.path());
        let mut res = Response::new();
        //res.header("Content-Type", "text/plain; charset=UTF-8")
        // .body(&(req.path().to_owned() + "\n"));
        res.header("Content-Type", "text/plain; charset=UTF-8")
         .body(&(req.path().to_owned() + " " + url.unwrap().path()));
        finished::<_, std::io::Error>(res)
    }).unwrap()
}

