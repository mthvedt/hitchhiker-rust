use std::fmt;

use thunderhead_store::{Source, Sink};

/// We use REST statuses throughout.
#[derive(Debug, Clone, Copy)]
pub enum TdStatus {
    Ok,
    Created,
    BadRequest,
    NotFound,
    InternalError,
}

impl TdStatus {
    fn to_http_str(&self) -> &'static str {
        match *self {
            TdStatus::Ok => "200 Ok",
            TdStatus::Created => "201 Created",
            TdStatus::BadRequest => "400 Bad Request",
            TdStatus::NotFound => "404 Not Found",
            TdStatus::InternalError => "500 Internal Error",
        }
    }
}

#[derive(Debug)]
pub struct RestError(TdStatus, String);

#[derive(Clone, Copy)]
pub enum TdMethod {
    GET,
    PUT,
    POST,
}

impl TdMethod {
    pub fn from_str(s: &str) -> Option<Self> {
        let r = match s {
            "GET" => TdMethod::GET,
            "PUT" => TdMethod::PUT,
            "POST" => TdMethod::POST,
            _ => return None,
        };

        Some(r)
    }

    pub fn to_str(&self) -> &'static str {
        match *self {
            TdMethod::GET => "GET",
            TdMethod::PUT => "PUT",
            TdMethod::POST => "POST",
        }
    }
}

impl fmt::Display for TdMethod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.to_str())
    }
}

impl fmt::Debug for TdMethod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        (self as &fmt::Display).fmt(f)
    }
}

pub trait HttpSource: Source<TdRest> {}
impl<S> HttpSource for S where S: Source<TdRest> {}

pub trait HttpSink: Sink<TdRest> + HttpSource {}
impl<S> HttpSink for S where S: Sink<TdRest> {}

/// A REST blob in Thunderhead. A Result<Option<TdRest>, TdError> gives us enough to construct an HTTP response.
#[derive(Debug)]
pub struct TdRest {
    // TODO: something fast that isn't a static str.
    mime_type: &'static str,
}

///
///
/// TODO: Right now, this makes a lot of copies and allocs. It shouldn't.
#[derive(Debug)]
pub struct TdRequest {
    // TODO: builders instead
    pub method: TdMethod,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

#[derive(Debug)]
pub enum TdRequestParse {
    Ok(TdRequest),
    Warn(TdRequest, String),
    Err(String),
}

#[derive(Debug)]
pub struct TdResponse {
    status: TdStatus,
    // TODO: we want this to stream
    body: Vec<u8>,
}

impl TdResponse {
    pub fn new(status: TdStatus) -> Self {
        TdResponse {
            status: status,
            body: Vec::new(),
        }
    }

    pub fn status_http_str(&self) -> &str {
        self.status.to_http_str()
    }

    pub fn with_body<Bytes: AsRef<[u8]>>(mut self, b: Bytes) -> Self {
        self.body = b.as_ref().into();
        self
    }

    /// TODO: this should really stream
    pub fn body(&self) -> &[u8] {
        self.body.as_ref()
    }
}
