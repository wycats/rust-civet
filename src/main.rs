#![feature(macro_rules)]

extern crate libc;
extern crate debug;
extern crate native;
extern crate collections;

use std::io::IoResult;
use civet::{Config,Server,Connection};

macro_rules! http_write(
    ($dst:expr, $fmt:expr $($arg:tt)*) => (
        try!(write!($dst, concat!($fmt, "\r\n") $($arg)*))
    )
)

mod civet;

fn main() {
    let _ = Server::start(Config { port: 8888, threads: 10 }, handler);

    loop {
        std::io::timer::sleep(1000);
    }
}

fn handler(mut conn: Connection) -> IoResult<()> {
    http_write!(conn, "HTTP/1.1 200 OK");
    http_write!(conn, "Content-Type: text/html");
    http_write!(conn, "");
    http_write!(conn, "<p>Method: {}</p>", conn.method());
    http_write!(conn, "<p>URL: {}</p>", conn.url());
    http_write!(conn, "<p>HTTP: {}</p>", conn.http_version());
    http_write!(conn, "<p>Remote IP: {}</p>", conn.remote_ip());
    http_write!(conn, "<p>Remote User: {}</p>", conn.remote_user());
    http_write!(conn, "<p>Query String: {}</p>", conn.query_string());
    http_write!(conn, "<p>SSL?: {}</p>", conn.is_ssl());
    http_write!(conn, "<p>Header Count: {}</p>", conn.count_headers());
    http_write!(conn, "<p>User Agent: {}</p>", conn.headers().find("User-Agent"));
    Ok(())

    //for (key, value) in conn.headers().iter() {
        //writeln!(conn, "<p>{} = {}</p>", key, value);
    //}
}
