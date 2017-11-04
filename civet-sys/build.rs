extern crate cc;

fn main() {
    cc::Build::new()
        .file("civetweb/src/civetweb.c")
        .include("civetweb/include")
        .compile("civetweb");
}
