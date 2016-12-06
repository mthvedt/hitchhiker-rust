//! Public for interface purposes.

use std::io;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle, Thread};

use futures::{self, Async, Poll, Stream, Sink};

// TODO: the control is inverted wrongly.
// Instead, TdHandles should be shared across all Thunderhead, so datastore/store futures know if they are cancelled.
struct HandleInner {
    term_flag: AtomicBool,
    kill_flag: AtomicBool,
    threads: Mutex<Vec<JoinHandle<()>>>,
    joiners: Mutex<Vec<Thread>>,
}

#[derive(Clone)]
pub struct Handle {
    inner: Arc<HandleInner>,
}

impl Handle {
    pub fn new() -> Self {
        Handle {
            inner: Arc::new(HandleInner {
                term_flag: AtomicBool::new(false),
                kill_flag: AtomicBool::new(false),
                threads: Mutex::new(Vec::new()),
                joiners: Mutex::new(Vec::new()),
            }),
        }
    }

    /// Terminates this handle. Dependent tasks must gracefully abort all operations.
    pub fn term(&self) {
        let mut joiners = self.inner.joiners.lock().unwrap();
        self.inner.term_flag.store(true, Ordering::SeqCst);

        for t in &*joiners {
            t.unpark();
        }
    }

    /// Tells if this handle is terminated.
    /// If true, dependent tasks must gracefully abort all operations.
    pub fn termed(&self) -> bool {
        self.inner.term_flag.load(Ordering::SeqCst)
    }

    /// Terminates and kills this handle. Dependent tasks must die as soon as is convenient.
    /// Thunderhead is intended to be 'crash only', and this is only used for testing. (TODO: enforce.)
    pub fn kill(&self) {
        let mut joiners = self.inner.joiners.lock().unwrap();
        self.term();
        self.inner.kill_flag.store(true, Ordering::SeqCst);

        for t in &*joiners {
            t.unpark();
        }
    }

    /// If true, any dependent task must die immediately.
    /// Thunderhead is intended to be 'crash only', and this is only used for testing. (TODO: enforce.)
    pub fn killed(&self) -> bool {
        self.inner.kill_flag.load(Ordering::SeqCst)
    }

    pub fn spawn<F: FnOnce() + Send + 'static>(&self, f: F) {
        if self.termed() {
            return
        }

        // Panic before spawning if we can't lock
        let mut threads = self.inner.threads.lock().unwrap();
        let h = thread::spawn(|| {
            // TODO catch, log, and die on panics
            f()
        });
        threads.push(h);
    }

    pub fn join(self) -> bool {
        loop {
            {
                let mut threads = self.inner.threads.lock().unwrap();
                let mut joiners = self.inner.joiners.lock().unwrap();

                match self.termed() {
                    true => {
                        let threads = &mut *threads;
                        while !threads.is_empty() {
                            threads.pop().unwrap().join();
                        }
                    },
                    false => joiners.push(thread::current()),
                }
            }

            // Exit mutex scope before parking!
            thread::park();
        }
    }
}

pub struct TransportWithHandle<T> {
    inner: T,
    handle: Handle,
}

impl<T> TransportWithHandle<T> {
    fn new(t: T, h: &Handle) -> Self {
        TransportWithHandle {
            inner: t,
            handle: h.clone(),
        }
    }
}

impl<T: Stream> Stream for TransportWithHandle<T> {
    type Item = T::Item;
    type Error = T::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.handle.termed() {
            true => Ok(Async::Ready(None)),
            false => self.inner.poll(),
        }
    }
}

impl<T: Sink<SinkError = io::Error>> Sink for TransportWithHandle<T> {
    type SinkItem = T::SinkItem;
    type SinkError = T::SinkError;

    fn start_send(&mut self, item: Self::SinkItem) -> futures::StartSend<Self::SinkItem, Self::SinkError> {
        match self.handle.killed() {
            true => self.inner.start_send(item),
            false => Err(io::Error::new(io::ErrorKind::ConnectionAborted, "transport terminated")),
        }
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        match self.handle.killed() {
            true => self.inner.poll_complete(),
            false => Err(io::Error::new(io::ErrorKind::ConnectionAborted, "transport terminated")),
        }
    }
}
