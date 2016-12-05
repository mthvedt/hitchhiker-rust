//! A package that implements a REST server for Thunderhead.
//!
//! This package should have its dependencies isolated, since tokio is unstable,
//! and we eventually want streaming reads/writes.
use std::io;
use std::io::Write;

use futures::{Future, finished};
use httparse;
use tokio_core::io::{Codec, EasyBuf, Framed, Io};
use tokio_proto::pipeline::ServerProto;
use tokio_service::Service;

use http::{TdMethod, TdRequest, TdRequestParse, TdResponse, TdStatus};

#[derive(Clone, Default)]
pub struct TdCodec;

impl TdCodec {
    // fn to_slice(a: &[u8], buf: &mut EasyBuf) -> (usize, )

    // Codec code taken from https://github.com/tokio-rs/tokio-minihttp/commit/b911096f5091958fd5a0d89ccdf29b67b29a88f3.
    // TODO: include relevant license.
    fn decode_helper(&mut self, buf: &mut EasyBuf) -> Result<Option<TdRequest>, String> {
        // TODO: we should grow this headers array if parsing fails and asks
        //       for more headers
        let mut headervec;
        let method;
        let path;
        let len;

        // Scoping block
        {
            let mut headers = [httparse::EMPTY_HEADER; 16];
            let mut r = httparse::Request::new(&mut headers);
            let status = r.parse(buf.as_slice());

            len = match status {
                Ok(httparse::Status::Complete(len)) => len,
                Ok(httparse::Status::Partial) => return Ok(None),
                // TODO: differentiate between invalid input and other io errors.
                Err(e) => return Err(format!("failed to parse http request: {:?}", e)),
            };

            // let toslice = |a: &[u8]| {
            //     let start = a.as_ptr() as usize - buf.as_slice().as_ptr() as usize;
            //     assert!(start < buf.len());
            //     (start, start + a.len())
            // };

            let tostring = |a: &[u8]| {
                let start = a.as_ptr() as usize - buf.as_slice().as_ptr() as usize;
                assert!(start < buf.len());
                String::from_utf8(buf.as_slice()[start..start + a.len()].into())
            };

            headervec = Vec::with_capacity(r.headers.len());
            for header in r.headers
                .iter()
                .map(|h| (tostring(h.name.as_bytes()), tostring(h.value)))
            {
                match header {
                    (Ok(k), Ok(v)) => headervec.push((k, v)),
                    (Err(e), _) => return Err(format!("Error parsing request {}: ", e)),
                    (_, Err(e)) => return Err(format!("Error parsing request {}: ", e)),
                }
            }

            let method_unparsed = r.method.unwrap();
            method = match TdMethod::from_str(method_unparsed) {
                Some(m) => m,
                None => return Err(format!("Bad method: {}", method_unparsed)),
            };

            path = String::from_utf8(r.path.unwrap().as_bytes().into()).unwrap();
        }

        Ok(Some(TdRequest {
            method: method,
            // TODO: are we sure this is always a string?
            path: path,
            headers: headervec,
            body: buf.drain_to(len).as_ref().into(),
        }))
    }
}

impl Codec for TdCodec {
    type In = TdRequestParse;
    type Out = TdResponse;

    // Codec code taken from https://github.com/tokio-rs/tokio-minihttp/commit/b911096f5091958fd5a0d89ccdf29b67b29a88f3.
    // TODO: include relevant license.
    fn decode(&mut self, buf: &mut EasyBuf) -> Result<Option<Self::In>, io::Error> {
        match self.decode_helper(buf) {
            Ok(opt_result) => Ok(opt_result.map(|r| TdRequestParse::Ok(r))),
            Err(s) => Ok(Some(TdRequestParse::Err(s))),
        }
    }

    fn encode(&mut self, resp: TdResponse, buf: &mut Vec<u8>) -> Result<(), io::Error> {
        write!(buf, "\
            HTTP/1.1 {} \r\n\
            Content-Length: {}\r\n\
        ", resp.status_http_str(), resp.body().len()).unwrap();

        // for &(ref k, ref v) in &msg.headers {
        //     buf.extend_from_slice(k.as_bytes());
        //     buf.extend_from_slice(b": ");
        //     buf.extend_from_slice(v.as_bytes());
        //     buf.extend_from_slice(b"\r\n");
        // }

        buf.extend_from_slice(b"\r\n");
        buf.extend_from_slice(resp.body());
        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct TdService;

impl Service for TdService {
    type Request = TdRequestParse;
    type Response = TdResponse;
    type Error = io::Error;
    type Future = Box<Future<Item = Self::Response, Error = io::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        println!("REQUEST: {:?}", req);

        // Create the HTTP response with the body
        let resp = TdResponse::new(TdStatus::Ok).with_body("this is my message");

        // Return the response as an immediate future
        finished(resp).boxed()
    }
}

pub struct TdProto;

impl<T: 'static + Io> ServerProto<T> for TdProto {
    type Request = TdRequestParse;
    type Response = TdResponse;
    type Error = io::Error;
    type Transport = Framed<T, TdCodec>;
    type BindTransport = Result<Self::Transport, io::Error>;

    fn bind_transport(&self, io: T) -> Self::BindTransport {
        Ok(io.framed(TdCodec))
    }
}
