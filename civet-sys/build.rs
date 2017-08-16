extern crate cmake;

use cmake::Config;

fn main() {
    let mut dst = Config::new("civetweb")
                         .define("CMAKE_BUILD_TYPE", "Release")
                         .define("BUILD_TESTING", "OFF")
                         .define("CIVETWEB_ALLOW_WARNINGS", "ON")
                         .build();
    dst.push("lib");
    println!("cargo:rustc-link-search=native={}", dst.display());
    println!("cargo:rustc-link-lib=static=civetweb");
}
