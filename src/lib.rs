#![feature(unsafe_destructor, old_io, std_misc, libc)]
#![cfg_attr(test, feature(io))]
#![allow(missing_copy_implementations)]

extern crate conduit;
extern crate libc;
extern crate semver;
extern crate "civet-sys" as ffi;

use std::old_io;
use std::old_io::net::ip::{IpAddr, Ipv4Addr};
use std::old_io::{IoResult, util, BufferedWriter};
use std::collections::HashMap;

use conduit::{Request, Handler, Extensions, TypeMap, Method, Scheme, Host};

use raw::{RequestInfo,Header};
use raw::{get_header,get_headers,get_request_info};
use status::{ToStatusCode};

pub use raw::Config;

mod raw;
pub mod status;

pub struct Connection<'a> {
    request: CivetRequest<'a>,
    written: bool,
}

pub struct CivetRequest<'a> {
    conn: &'a raw::Connection,
    request_info: RequestInfo<'a>,
    headers: Headers<'a>,
    extensions: Extensions
}

fn ver(major: u64, minor: u64) -> semver::Version {
    semver::Version {
        major: major,
        minor: minor,
        patch: 0,
        pre: vec!(),
        build: vec!()
    }
}

impl<'a> conduit::Request for CivetRequest<'a> {
    fn http_version(&self) -> semver::Version {
        let version = self.request_info.http_version().unwrap();
        match version {
            "1.0" => ver(1, 0),
            "1.1" => ver(1, 1),
            _ => ver(1, 1)
        }
    }

    fn conduit_version(&self) -> semver::Version {
        ver(0, 1)
    }

    fn method(&self) -> Method {
        match self.request_info.method().unwrap() {
            "HEAD" => Method::Head,
            "GET" => Method::Get,
            "POST" => Method::Post,
            "PUT" => Method::Put,
            "DELETE" => Method::Delete,
            "PATCH" => Method::Patch,
            "PURGE" => Method::Purge,
            "CONNECT" => Method::Connect,
            "OPTIONS" => Method::Options,
            "TRACE" => Method::Trace,
            other @ _ => panic!("Civet does not support {} requests", other)
        }
    }

    fn scheme(&self) -> Scheme {
        if self.request_info.is_ssl() {
            Scheme::Https
        } else {
            Scheme::Http
        }
    }

    fn host(&self) -> Host {
        Host::Name(get_header(self.conn, "Host").unwrap())
    }

    fn virtual_root(&self) -> Option<&str> {
        None
    }

    fn path(&self) -> &str {
        self.request_info.url().unwrap()
    }

    fn query_string(&self) -> Option<&str> {
        self.request_info.query_string()
    }

    fn remote_ip(&self) -> IpAddr {
        let ip = self.request_info.remote_ip();
        Ipv4Addr((ip >> 24) as u8,
                 (ip >> 16) as u8,
                 (ip >>  8) as u8,
                 (ip >>  0) as u8)
    }

    fn content_length(&self) -> Option<u64> {
        get_header(self.conn, "Content-Length").and_then(|s| s.parse().ok())
    }

    fn headers(&self) -> &conduit::Headers {
        &self.headers as &conduit::Headers
    }

    fn body(&mut self) -> &mut Reader {
        self as &mut Reader
    }

    fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    fn mut_extensions(&mut self) -> &mut Extensions {
        &mut self.extensions
    }
}

pub fn response<S: ToStatusCode, R: Reader + Send + 'static>(status: S,
    headers: HashMap<String, Vec<String>>, body: R) -> conduit::Response
{
    conduit::Response {
        status: status.to_status().ok().unwrap().to_code(),
        headers: headers,
        body: Box::new(body),
    }
}

impl<'a> Connection<'a> {
    fn new(conn: &raw::Connection) -> Result<Connection, String> {
        match request_info(conn) {
            Ok(info) => {
                let request = CivetRequest {
                    conn: conn,
                    request_info: info,
                    headers: Headers { conn: conn },
                    extensions: TypeMap::new()
                };

                Ok(Connection {
                    request: request,
                    written: false,
                })
            },
            Err(err) => Err(err)
        }
    }

}

impl<'a> Writer for Connection<'a> {
    fn write_all(&mut self, buf: &[u8]) -> IoResult<()> {
        self.written = true;
        write_bytes(self.request.conn, buf).map_err(|_| {
            old_io::standard_error(old_io::IoUnavailable)
        })
    }
}

impl<'a> Reader for CivetRequest<'a> {
    fn read(&mut self, buf: &mut[u8]) -> IoResult<usize> {
        let ret = raw::read(self.conn, buf);

        if ret == 0 {
            Err(old_io::standard_error(old_io::EndOfFile))
        } else {
            Ok(ret as usize)
        }
    }
}

#[unsafe_destructor]
impl<'a> Drop for Connection<'a> {
    fn drop(&mut self) {
        if !self.written {
            let _ = writeln!(self, "HTTP/1.1 500 Internal Server Error");
        }
    }
}

pub struct Headers<'a> {
    conn: &'a raw::Connection
}

impl<'a> conduit::Headers for Headers<'a> {
    fn find(&self, string: &str) -> Option<Vec<&str>> {
        get_header(self.conn, string).map(|s| vec!(s))
    }

    fn has(&self, string: &str) -> bool {
        get_header(self.conn, string).is_some()
    }

    fn all(&self) -> Vec<(&str, Vec<&str>)> {
        HeaderIterator::new(self.conn).collect()
    }
}

pub struct HeaderIterator<'a> {
    headers: Vec<Header<'a>>,
    position: usize
}

impl<'a> HeaderIterator<'a> {
    fn new<'b>(conn: &'b raw::Connection) -> HeaderIterator<'b> {
        HeaderIterator { headers: get_headers(conn), position: 0 }
    }
}

impl<'a> Iterator for HeaderIterator<'a> {
    type Item = (&'a str, Vec<&'a str>);
    fn next(&mut self) -> Option<(&'a str, Vec<&'a str>)> {
        let pos = self.position;
        let headers = &self.headers;

        if self.headers.len() <= pos {
            None
        } else {
            let header = &headers[pos];
            self.position += 1;
            header.name().map(|name| (name, vec!(header.value().unwrap())))
        }
    }
}

pub struct Server(raw::Server<Box<Handler + 'static + Sync>>);

impl Server {
    pub fn start<H: Handler + 'static + Sync>(options: Config, handler: H)
        -> IoResult<Server>
    {
        fn internal_handler(conn: &mut raw::Connection,
                            handler: &Box<Handler + 'static + Sync>)
                            -> Result<(), ()> {
            let mut connection = Connection::new(conn).unwrap();
            let response = handler.call(&mut connection.request);
            let mut writer = BufferedWriter::new(connection);

            fn err<W: Writer>(writer: &mut W) {
                let _ = writeln!(writer, "HTTP/1.1 500 Internal Server Error");
            }

            let conduit::Response { status, headers, mut body } = match response {
                Ok(r) => r,
                Err(_) => return Err(err(&mut writer)),
            };
            let (code, string) = status;
            try!(write!(&mut writer, "HTTP/1.1 {} {}\r\n", code, string).map_err(|_| ()));

            for (key, value) in headers.iter() {
                for header in value.iter() {
                    try!(write!(&mut writer, "{}: {}\r\n", *key, *header).map_err(|_| ()));
                }
            }

            try!(write!(&mut writer, "\r\n").map_err(|_| ()));
            let mut body: &mut Reader = &mut *body;
            try!(util::copy(&mut body, &mut writer).map_err(|_| ()));

            Ok(())
        }

        let handler = Box::new(handler);
        let raw_callback = raw::ServerCallback::new(internal_handler, handler);
        Ok(Server(try!(raw::Server::start(options, raw_callback))))
    }
}

fn write_bytes(connection: &raw::Connection, bytes: &[u8]) -> Result<(), String> {
    let ret = raw::write(connection, bytes);

    if ret == -1 {
        return Err("Couldn't write bytes to the connection".to_string())
    }

    Ok(())
}

fn request_info<'a>(connection: &'a raw::Connection)
    -> Result<RequestInfo<'a>, String>
{
    match get_request_info(connection) {
        Some(info) => Ok(info),
        None => Err("Couldn't get request info for connection".to_string())
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;
    use std::error::Error;
    use std::old_io::net::ip::SocketAddr;
    use std::old_io::net::tcp::TcpStream;
    use std::old_io::test::next_test_ip4;
    use std::old_io::{MemReader};
    use std::sync::Mutex;
    use std::io;
    use std::sync::mpsc::{channel, Sender};
    use super::{Server, Config, response};
    use conduit::{Request, Response, Handler};

    fn noop(_: &mut Request) -> Result<Response, io::Error> { unreachable!() }

    fn request(addr: SocketAddr, req: &str) -> String {
        let mut s = TcpStream::connect(addr).unwrap();
        s.write_str(req.trim_left()).unwrap();
        s.read_to_string().unwrap()
    }

    #[test]
    fn smoke() {
        let addr = next_test_ip4();
        Server::start(Config { port: addr.port, threads: 1 }, noop).unwrap();
    }

    #[test]
    fn dupe_port() {
        let addr = next_test_ip4();
        let s1 = Server::start(Config { port: addr.port, threads: 1 }, noop);
        assert!(s1.is_ok());
        let s2 = Server::start(Config { port: addr.port, threads: 1 }, noop);
        assert!(s2.is_err());
    }

    #[test]
    fn drops_handler() {
        static mut DROPPED: bool = false;
        struct Foo;
        impl Handler for Foo {
            fn call(&self, _req: &mut Request) -> Result<Response, Box<Error+Send>> {
                panic!()
            }
        }
        impl Drop for Foo {
            fn drop(&mut self) { unsafe { DROPPED = true; } }
        }

        let addr = next_test_ip4();
        drop(Server::start(Config { port: addr.port, threads: 1 }, Foo));
        unsafe { assert!(DROPPED); }
    }

    #[test]
    fn invokes() {
        struct Foo(Mutex<Sender<()>>);
        impl Handler for Foo {
            fn call(&self, _req: &mut Request) -> Result<Response, Box<Error+Send>> {
                let Foo(ref tx) = *self;
                tx.lock().unwrap().send(()).unwrap();
                Ok(response(200, HashMap::new(), MemReader::new(vec![])))
            }
        }

        let addr = next_test_ip4();
        let (tx, rx) = channel();
        let handler = Foo(Mutex::new(tx));
        let _s = Server::start(Config { port: addr.port, threads: 1 }, handler);
        request(addr, r"
GET / HTTP/1.1

");
        rx.recv().unwrap();
    }

    #[test]
    fn header_sent() {
        struct Foo(Mutex<Sender<String>>);
        impl Handler for Foo {
            fn call(&self, req: &mut Request) -> Result<Response, Box<Error+Send>> {
                let Foo(ref tx) = *self;
                tx.lock().unwrap()
                  .send(req.headers().find("Foo").unwrap().connect("")).unwrap();
                Ok(response(200, HashMap::new(), MemReader::new(vec![])))
            }
        }

        let addr = next_test_ip4();
        let (tx, rx) = channel();
        let handler = Foo(Mutex::new(tx));
        let _s = Server::start(Config { port: addr.port, threads: 1 }, handler);
        request(addr, r"
GET / HTTP/1.1
Foo: bar

");
        assert_eq!(rx.recv().unwrap().as_slice(), "bar");
    }

    #[test]
    fn failing_handler() {
        struct Foo;
        impl Handler for Foo {
            fn call(&self, _req: &mut Request) -> Result<Response, Box<Error+Send>> {
                panic!()
            }
        }

        let addr = next_test_ip4();
        let _s = Server::start(Config { port: addr.port, threads: 1 }, Foo);
        request(addr, r"
GET / HTTP/1.1
Foo: bar

");
    }

    #[test]
    fn failing_handler_is_500() {
        struct Foo;
        impl Handler for Foo {
            fn call(&self, _req: &mut Request) -> Result<Response, Box<Error+Send>> {
                panic!()
            }
        }

        let addr = next_test_ip4();
        let _s = Server::start(Config { port: addr.port, threads: 1 }, Foo);
        let response = request(addr, r"
GET / HTTP/1.1
Foo: bar

");
        assert!(response.as_slice().contains("500 Internal"),
                "not a failing response: {}", response);
    }
}
