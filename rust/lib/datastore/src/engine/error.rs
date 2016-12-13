use std::io::{Write, stderr};

use libc::c_uint;

// TODO: a better story for exceptions and errors.

#[derive(Debug)]
pub struct Exception {
    pub message: String,
}

pub enum ErrorType {
    Error = 0x0,
    Warning = 0x1,
    UncaughtException = 0x2,
}

// TODO: should not be spidermonkey-specific
// TODO: pretty-printing for errors
// TODO: unify error and exception
#[derive(Debug)]
pub struct Error {
    pub filename: String,
    pub line: String,
    pub lineno: c_uint,
    pub column: c_uint,
    pub is_muted: bool,
    // TODO: why two messages?
    pub message: String,
    pub message2: String,
    pub js_flags: c_uint,
    pub js_errno: c_uint,
    pub js_exntype: i16,
}

pub trait ErrorReporter {
    fn is_empty(&self) -> bool;
    fn report_warning(&mut self, e: Error);
    fn report_exception(&mut self, ex: Exception);
}

/// An error reporter that logs to stderr. It never takes ownership of errors, so it is always empty.
/// Useful for bootstrapping.
// TODO: should probably log to a master 'TDContext' instead.
pub struct LoggingErrorReporter;

impl ErrorReporter for LoggingErrorReporter {
    fn is_empty(&self) -> bool {
        true
    }

    fn report_warning(&mut self, e: Error) {
        writeln!(&mut stderr(), "{:?}", e).ok().unwrap();
    }

    fn report_exception(&mut self, ex: Exception) {
        writeln!(&mut stderr(), "{:?}", ex).ok().unwrap();
    }
}

// // TODO: eventually, this should be configurable
// const ERROR_QUEUE_MAX: usize = 20;
//
// struct ErrorQueue {
//     warnings: Vec<Error>,
//     extra_warnings: usize,
//     errors: Vec<Error>,
//     extra_errors: usize,
// }
//
// impl ErrorQueue {
//     fn new() -> Self {
//         ErrorQueue {
//             warnings: Vec::with_capacity(ERROR_QUEUE_MAX),
//             extra_warnings: 0,
//             errors: Vec::with_capacity(ERROR_QUEUE_MAX),
//             extra_errors: 0,
//         }
//     }
//
//     fn is_empty(&self) -> bool {
//         self.inner.len() == 0
//     }
//
//     fn drain_errors(&mut self) -> Vec<Error> {
//         let mut r = Vec::with_capacity(ERROR_QUEUE_MAX);
//         mem::swap(&mut r, &mut self.errors);
//         self.saturated = false;
//
//         r
//     }
// }
//
// impl ErrorReporter for ErrorQueue {
//     fn push(&mut self, e: Error) {
//         if self.inner.len() > ERROR_QUEUE_MAX {
//             self.saturated = true;
//         } else if is_error {
//             self.errors.push(e);
//         }
//     }
// }
