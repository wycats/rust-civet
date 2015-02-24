#![feature(fs, path, process, env)]

use std::env;
use std::fs;
use std::process::Command;
use std::path::Path;

fn main() {
    let dst = env::var("OUT_DIR").unwrap();

    assert!(Command::new("make")
                    .current_dir("civetweb")
                    .arg("lib")
                    .arg(&format!("BUILD_DIR={}", dst))
                    .env("COPT", "-fPIC")
                    .status().unwrap().success());

    {
        let src = Path::new("civetweb/libcivetweb.a");
        let dst = Path::new(&dst).join("libcivetweb.a");
        if fs::rename(&src, &dst).is_err() {
            fs::copy(&src, &dst).unwrap();
            fs::remove_file(&src).unwrap();
        }
    }

    println!("cargo:rustc-flags=-L {} -l civetweb:static", dst);
}
