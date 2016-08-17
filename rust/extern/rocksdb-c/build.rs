use std::process::Command;
use std::env;
//use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    // Builds native only.
    // TODO: use Path, not unix paths.
    Command::new("make")
        .current_dir("remote/")
        .args(&["static_lib"])
        .status().unwrap();
    Command::new("cp")
        .args(&["remote/librocksdb.a", &out_dir])
        .status().unwrap();

    println!("cargo:rustc-link-search=native={}", &out_dir);
    // println!("cargo:warning=rocksdb is native-only", out_dir);
}

