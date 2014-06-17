#![feature(macro_rules)]

extern crate civet;

use std::io::{IoResult,MemReader,MemWriter};
use std::collections::HashMap;

use civet::{Config,Server,Request,Response};

macro_rules! http_write(
    ($dst:expr, $fmt:expr $($arg:tt)*) => (
        try!(write!($dst, concat!($fmt, "\r\n") $($arg)*))
    )
)

fn main() {
    let _a = Server::start(Config { port: 8888, threads: 50 }, handler);

    loop {
        std::io::timer::sleep(1000);
    }
}

fn handler(req: &mut Request) -> IoResult<Response<int, MemReader>> {
    let mut res = MemWriter::with_capacity(10000);

    http_write!(res, "<style>body {{ font-family: sans-serif; }}</style>");
    http_write!(res, "<p>Method: {}</p>", req.method());
    http_write!(res, "<p>URL: {}</p>", req.url());
    http_write!(res, "<p>HTTP: {}</p>", req.http_version());
    http_write!(res, "<p>Remote IP: {}</p>", req.remote_ip());
    http_write!(res, "<p>Remote Port: {}</p>", req.remote_port());
    http_write!(res, "<p>Remote User: {}</p>", req.remote_user());
    http_write!(res, "<p>Query String: {}</p>", req.query_string());
    http_write!(res, "<p>SSL?: {}</p>", req.is_ssl());
    http_write!(res, "<p>Header Count: {}</p>", req.count_headers());
    http_write!(res, "<p>User Agent: {}</p>", req.headers().find("User-Agent"));
    http_write!(res, "<p>Input: {}</p>", try!(req.read_to_str()));

    http_write!(res, "<h2>Headers</h2><ul>");

    for (key, value) in req.headers().iter() {
        http_write!(res, "<li>{} = {}</li>", key, value);
    }

    http_write!(res, "</ul>");

    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_str(), "text/html".to_str());

    let body = MemReader::new(res.unwrap());

    Ok(Response::new(200, headers, body))
}