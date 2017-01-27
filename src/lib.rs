extern crate conduit;
extern crate libc;
extern crate semver;
extern crate civet_sys as ffi;

use std::collections::HashMap;
use std::io::prelude::*;
use std::io::{self, BufWriter};
use std::net::{SocketAddr, Ipv4Addr, SocketAddrV4};

use conduit::{Handler, Extensions, TypeMap, Method, Scheme, Host};

use raw::{RequestInfo,Header};
use raw::{get_header,get_headers,get_request_info};
use status::{ToStatusCode};

pub use config::Config;

mod raw;
mod config;
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

    fn remote_addr(&self) -> SocketAddr {
        let ip = self.request_info.remote_ip();
        let ip = Ipv4Addr::new((ip >> 24) as u8,
                               (ip >> 16) as u8,
                               (ip >>  8) as u8,
                               (ip >>  0) as u8);
        SocketAddr::V4(SocketAddrV4::new(ip, self.request_info.remote_port()))
    }

    fn content_length(&self) -> Option<u64> {
        get_header(self.conn, "Content-Length").and_then(|s| s.parse().ok())
    }

    fn headers(&self) -> &conduit::Headers { &self.headers }

    fn body(&mut self) -> &mut Read { self }

    fn extensions(&self) -> &Extensions { &self.extensions }

    fn mut_extensions(&mut self) -> &mut Extensions {
        &mut self.extensions
    }
}

pub fn response<S: ToStatusCode, R: Read + Send + 'static>(status: S,
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

impl<'a> Write for Connection<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.written = true;
        match raw::write(self.request.conn, buf) {
            n if n < 0 => Err(io::Error::new(io::ErrorKind::Other,
                                             &format!("write error ({})", n)[..])),
            n => Ok(n as usize)
        }
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

impl<'a> Read for CivetRequest<'a> {
    fn read(&mut self, buf: &mut[u8]) -> io::Result<usize> {
        match raw::read(self.conn, buf) {
            n if n < 0 => Err(io::Error::new(io::ErrorKind::Other,
                                             &format!("read error ({})", n)[..])),
            n => Ok(n as usize)
        }
    }
}

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
        -> io::Result<Server>
    {
        fn internal_handler(conn: &mut raw::Connection,
                            handler: &Box<Handler + 'static + Sync>)
                            -> Result<(), ()> {
            let mut connection = Connection::new(conn).unwrap();
            let response = handler.call(&mut connection.request);
            let mut writer = BufWriter::new(connection);

            fn err<W: Write>(writer: &mut W) {
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
            try!(body.write_body(&mut writer).map_err(|_| ()));

            Ok(())
        }

        let handler = Box::new(handler);
        let raw_callback = raw::ServerCallback::new(internal_handler, handler);
        Ok(Server(try!(raw::Server::start(options, raw_callback))))
    }
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
    use std::io::prelude::*;
    use std::io::{self, Cursor};
    use std::net::{SocketAddr, TcpStream, SocketAddrV4, Ipv4Addr};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};
    use std::sync::mpsc::{channel, Sender};
    use super::{Server, Config, response};
    use conduit::{Request, Response, Handler};

    fn noop(_: &mut Request) -> Result<Response, io::Error> { unreachable!() }

    fn request(addr: SocketAddr, req: &str) -> String {
        let mut s = TcpStream::connect(&addr).unwrap();
        s.write_all(req.trim_left().as_bytes()).unwrap();
        let mut ret = String::new();
        s.read_to_string(&mut ret).unwrap();
        ret
    }

    fn port() -> u16 {
        static CNT: AtomicUsize = ATOMIC_USIZE_INIT;
        CNT.fetch_add(1, Ordering::SeqCst) as u16 + 13038
    }

    fn cfg(port: u16) -> Config {
        let mut cfg = Config::new();
        cfg.port(port).threads(1);
        return cfg
    }

    #[test]
    fn smoke() {
        Server::start(cfg(port()), noop).unwrap();
    }

    #[test]
    fn dupe_port() {
        let port = port();
        let s1 = Server::start(cfg(port), noop);
        assert!(s1.is_ok());
        let s2 = Server::start(cfg(port), noop);
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

        drop(Server::start(cfg(port()), Foo));
        unsafe { assert!(DROPPED); }
    }

    #[test]
    fn invokes() {
        struct Foo(Mutex<Sender<()>>);
        impl Handler for Foo {
            fn call(&self, _req: &mut Request) -> Result<Response, Box<Error+Send>> {
                let Foo(ref tx) = *self;
                tx.lock().unwrap().send(()).unwrap();
                Ok(response(200, HashMap::new(), Cursor::new(vec![])))
            }
        }

        let (tx, rx) = channel();
        let handler = Foo(Mutex::new(tx));
        let port = port();
        let ip = Ipv4Addr::new(127, 0, 0, 1);
        let addr = SocketAddr::V4(SocketAddrV4::new(ip, port));
        let _s = Server::start(cfg(port), handler);
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
                  .send(req.headers().find("Foo").unwrap().join("")).unwrap();
                Ok(response(200, HashMap::new(), Cursor::new(vec![])))
            }
        }

        let (tx, rx) = channel();
        let handler = Foo(Mutex::new(tx));
        let port = port();
        let ip = Ipv4Addr::new(127, 0, 0, 1);
        let addr = SocketAddr::V4(SocketAddrV4::new(ip, port));
        let _s = Server::start(cfg(port), handler);
        request(addr, r"
GET / HTTP/1.1
Foo: bar

");
        assert_eq!(rx.recv().unwrap(), "bar");
    }

    #[test]
    fn failing_handler() {
        struct Foo;
        impl Handler for Foo {
            fn call(&self, _req: &mut Request) -> Result<Response, Box<Error+Send>> {
                panic!()
            }
        }

        let port = port();
        let ip = Ipv4Addr::new(127, 0, 0, 1);
        let addr = SocketAddr::V4(SocketAddrV4::new(ip, port));
        let _s = Server::start(cfg(port), Foo);
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

        let port = port();
        let ip = Ipv4Addr::new(127, 0, 0, 1);
        let addr = SocketAddr::V4(SocketAddrV4::new(ip, port));
        let _s = Server::start(cfg(port), Foo);
        let response = request(addr, r"
GET / HTTP/1.1
Foo: bar

");
        assert!(response.contains("500 Internal"),
                "not a failing response: {}", response);
    }
}
