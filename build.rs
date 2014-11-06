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

    {
        let src = Path::new("src/civetweb/libcivetweb.a");
        let dst = Path::new(&dst).join("libcivetweb.a");
        if fs::rename(&src, &dst).is_err() {
            fs::copy(&src, &dst).unwrap();
            fs::unlink(&src).unwrap();
        }
    }

    println!("cargo:rustc-flags=-L {} -l civetweb:static", dst);
}
