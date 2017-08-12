extern crate cmake;

use cmake::Config;

fn main() {
    let mut dst = Config::new("civetweb")
                         .define("CMAKE_BUILD_TYPE", "Release")
                         .build();
    dst.push("lib");
    println!("cargo:rustc-link-search=native={}", dst.display());
    println!("cargo:rustc-link-lib=static=civetweb");
}
