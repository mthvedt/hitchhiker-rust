//! thread_local perf test

#![feature(test)]

extern crate time;
use std::cell::Cell;

extern crate test;
use test::black_box;

const ITERATION : i64 = 5000000;

fn main() {
    unsafe {c_thread_local();}
    c_version();
    rust_version();
}

#[link(name = "thread_local", kind = "static")]
extern "C" {
    fn create_thread_local();
    fn thread_local() -> *mut ThreadLocal;
    fn c_thread_local();
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ThreadLocal {
    i : i64
}

fn c_version() {
    unsafe {
        create_thread_local();

        let start = time::now_utc();

        for i in 1..ITERATION {
            let i0 = (*thread_local()).i;
            (*thread_local()).i = i0 + i;
            black_box(i0);
        }
        let end = time::now_utc();

        println!("Rust_C: {}", (*thread_local()).i);
        println!("Rust_C: {} msec", (end - start).num_milliseconds());
    }   
}

thread_local!(static RUST_THREAD_LOCAL : Cell<ThreadLocal> = Cell::new(ThreadLocal{i: 0}));

fn rust_version() {
    let start = time::now_utc();

    for i in 1..ITERATION {
        RUST_THREAD_LOCAL.with(|x| {
            let i0 = x.get().i;
            x.set(ThreadLocal { i: i0 + i });
            black_box(i0);
        });
    }

    let end = time::now_utc();

    println!("RUST: {}", RUST_THREAD_LOCAL.with(|x| {x.get().i}));
    println!("RUST: {} msec", (end - start).num_milliseconds()); 
}
