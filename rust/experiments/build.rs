extern crate gcc;

fn main() {
    gcc::Config::new()
        .file("src/c/thread_local.c")
        .include("src")
        .compile("libthread_local.a");
}
