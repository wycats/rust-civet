#![feature(io)]

extern crate civet;
extern crate conduit;

use std::collections::HashMap;
use std::sync::mpsc::channel;
use std::io::{self, Cursor};
use std::io::prelude::*;

use civet::{Config, Server, response};
use conduit::{Request, Response};

macro_rules! http_write {
    ($dst:expr, $fmt:expr) => (
        try!(write!(&mut $dst, concat!($fmt, "\r\n")))
    );
    ($dst:expr, $fmt:expr, $($arg:tt)*) => (
        try!(write!(&mut $dst, concat!($fmt, "\r\n"), $($arg)*))
    )
}

fn main() {
    let _a = Server::start(Config { port: 8888, threads: 50 }, handler);
    let (_tx, rx) = channel::<()>();
    rx.recv().unwrap();
}

fn handler(req: &mut Request) -> io::Result<Response> {
    let mut res = Cursor::new(Vec::with_capacity(10000));

    http_write!(res, "<style>body {{ font-family: sans-serif; }}</style>");
    http_write!(res, "<p>HTTP {}</p>", req.http_version());
    http_write!(res, "<p>Method: {:?}</p>", req.method());
    http_write!(res, "<p>Scheme: {:?}</p>", req.scheme());
    http_write!(res, "<p>Host: {:?}</p>", req.host());
    http_write!(res, "<p>Path: {}</p>", req.path());
    http_write!(res, "<p>Query String: {:?}</p>", req.query_string());
    http_write!(res, "<p>Remote IP: {}</p>", req.remote_ip());
    http_write!(res, "<p>Content Length: {:?}</p>", req.content_length());

    let mut body = String::new();
    req.body().read_to_string(&mut body).unwrap();
    http_write!(res, "<p>Input: {}", body);

    http_write!(res, "<h2>Headers</h2><ul>");

    // for (key, value) in req.headers().iter() {
    //     http_write!(res, "<li>{} = {}</li>", key, value);
    // }

    http_write!(res, "</ul>");

    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), vec!("text/html".to_string()));

    let body = Cursor::new(res.into_inner());

    Ok(response(200, headers, body))
}
