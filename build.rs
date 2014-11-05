use std::os;
use std::io::{fs, Command};
use std::io::process::InheritFd;

fn main() {
    let dst = os::getenv("OUT_DIR").unwrap();

    assert!(Command::new("make")
                    .cwd(&Path::new("src/civetweb"))
                    .arg("lib")
                    .arg(format!("BUILD_DIR={}", dst))
                    .env("COPT", "-fPIC")
                    .stdout(InheritFd(1))
                    .stderr(InheritFd(2))
                    .status().unwrap().success());

    fs::rename(&Path::new("src/civetweb/libcivetweb.a"),
               &Path::new(&dst).join("libcivetweb.a")).unwrap();

    println!("cargo:rustc-flags=-L {} -l civetweb:static", dst);
}
