#![feature(macro_rules)]

extern crate civet;
extern crate conduit;

use std::io::{IoResult, MemReader, MemWriter};
use std::collections::HashMap;

use civet::{Config, Server, response};
use conduit::{Request, Response};

macro_rules! http_write(
    ($dst:expr, $fmt:expr $($arg:tt)*) => (
        try!(write!($dst, concat!($fmt, "\r\n") $($arg)*))
    )
)

fn main() {
    let _a = Server::start(Config { port: 8888, threads: 50 }, handler);
    let (_tx, rx) = channel::<()>();
    rx.recv();
}

fn handler(req: &mut Request) -> IoResult<Response> {
    let mut res = MemWriter::with_capacity(10000);

    http_write!(res, "<style>body {{ font-family: sans-serif; }}</style>");
    http_write!(res, "<p>HTTP {}</p>", req.http_version());
    http_write!(res, "<p>Method: {}</p>", req.method());
    http_write!(res, "<p>Scheme: {}</p>", req.scheme());
    http_write!(res, "<p>Host: {}</p>", req.host());
    http_write!(res, "<p>Path: {}</p>", req.path());
    http_write!(res, "<p>Query String: {}</p>", req.query_string());
    http_write!(res, "<p>Remote IP: {}</p>", req.remote_ip());
    http_write!(res, "<p>Content Length: {}</p>", req.content_length());

    http_write!(res, "<p>Input: {}", req.body().read_to_string());

    http_write!(res, "<h2>Headers</h2><ul>");

    // for (key, value) in req.headers().iter() {
    //     http_write!(res, "<li>{} = {}</li>", key, value);
    // }

    http_write!(res, "</ul>");

    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), vec!("text/html".to_string()));

    let body = MemReader::new(res.unwrap());

    Ok(response(200i, headers, body))
}
