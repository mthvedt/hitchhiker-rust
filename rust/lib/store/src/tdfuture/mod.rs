//! Thunderhead futures.
//!
//! Futures in Thunderhead are asynchronous computations that access shared context.
//!
//! Thunderhead futures is currently built on the futures-rs and tokio-rs libraries.
//! We wrap these libraries for a few reasons:
//!
//! * they are unstable,
//! * we want our futures to handle threading context in a non-clumsy way,
//! * futures-rs lacks a good story to handle loops/recursion,
//! * we want to limit ourselves to a subset of future functionality, for clarity and perf's sake,
//! * futures-rs has a few more indirections than necessary (though it is still very fast),
//! * we want to uniformly represent all Thunderhead tasks as a single boxed future type,
//! * we eventually want to add our own bells and whistles.

// mod future;
// pub use self::future::*;

mod loops;
pub use self::loops::*;

mod spin;
pub use self::spin::*;
