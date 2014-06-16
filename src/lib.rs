#![crate_id="civet"]
#![crate_type="rlib"]

extern crate libc;
extern crate debug;
extern crate native;
extern crate collections;

use std::io;
use std::io::{IoResult,util};
use std::collections::HashMap;

use raw::{RequestInfo,Header};
use raw::{get_header,get_headers,get_request_info};
use status::{ToStatusCode};

pub use raw::Config;

mod raw;
pub mod status;

pub struct Connection<'a> {
    request: Request<'a>
}

pub struct Request<'a> {
    conn: &'a raw::Connection,
    request_info: RequestInfo<'a>
}

pub struct CopiedRequest {
    pub headers: HashMap<String, String>,
    pub method: String,
    pub url: String,
    pub http_version: String,
    pub query_string: String,
    pub remote_user: String,
    pub remote_ip: int,
    pub remote_port: int,
    pub is_ssl: bool
}

impl<'a> Request<'a> {
    pub fn copy(&self) -> CopiedRequest {
        let mut headers = HashMap::new();

        for (key, value) in self.headers().iter() {
            headers.insert(key.to_str(), value.to_str());
        }

        CopiedRequest {
            headers: headers,
            method: self.method().to_str(),
            url: self.url().to_str(),
            http_version: self.http_version().to_str(),
            query_string: self.query_string().to_str(),
            remote_user: self.remote_user().to_str(),
            remote_ip: self.remote_ip(),
            remote_port: self.remote_port(),
            is_ssl: self.is_ssl()
        }
    }

    pub fn get_header<S: Str>(&mut self, string: S) -> Option<String> {
        get_header(self.conn, string.as_slice())
    }

    pub fn count_headers(&self) -> uint {
        self.request_info.num_headers() as uint
    }

    pub fn method<'a>(&'a self) -> Option<&'a str> {
        self.request_info.method()
    }

    pub fn url<'a>(&'a self) -> Option<&'a str> {
        self.request_info.url()
    }

    pub fn http_version<'a>(&'a self) -> Option<&'a str> {
        self.request_info.http_version()
    }

    pub fn query_string<'a>(&'a self) -> Option<&'a str> {
        self.request_info.query_string()
    }

    pub fn remote_user<'a>(&'a self) -> Option<&'a str> {
        self.request_info.remote_user()
    }

    pub fn remote_ip(&self) -> int {
        self.request_info.remote_ip()
    }

    pub fn remote_port(&self) -> int {
        self.request_info.remote_port()
    }

    pub fn is_ssl(&self) -> bool {
        self.request_info.is_ssl()
    }

    pub fn headers<'a>(&'a self) -> Headers<'a> {
        Headers { conn: self.conn }
    }
}

pub struct Response<S, R> {
    status: S,
    headers: HashMap<String, String>,
    body: R
}

impl<S: ToStatusCode, R: Reader + Send> Response<S, R> {
    pub fn new(status: S, headers: HashMap<String, String>, body: R) -> Response<S, R> {
        Response { status: status, headers: headers, body: body }
    }
}

impl<'a> Connection<'a> {
    fn new<'a>(conn: &'a raw::Connection) -> Result<Connection<'a>, String> {
        match request_info(conn) {
            Ok(info) => {
                let request = Request { conn: conn, request_info: info };
                Ok(Connection {
                    request: request
                })
            },
            Err(err) => Err(err)
        }
    }

}

impl<'a> Writer for Connection<'a> {
    fn write(&mut self, buf: &[u8]) -> IoResult<()> {
        write_bytes(self.request.conn, buf).map_err(|_| {
            io::standard_error(io::IoUnavailable)
        })
    }
}

impl<'a> Reader for Request<'a> {
    fn read(&mut self, buf: &mut[u8]) -> IoResult<uint> {
        let ret = raw::read(self.conn, buf);

        if ret == 0 {
            Err(io::standard_error(io::EndOfFile))
        } else {
            Ok(ret as uint)
        }
    }
}

pub struct Headers<'a> {
    conn: &'a raw::Connection
}

impl<'a> Headers<'a> {
    pub fn find<S: Str>(&self, string: S) -> Option<String> {
        get_header(self.conn, string.as_slice())
    }

    pub fn iter<'a>(&'a self) -> HeaderIterator<'a> {
        HeaderIterator::new(self.conn)
    }
}

pub struct HeaderIterator<'a> {
    headers: Vec<Header<'a>>,
    position: uint
}

impl<'a> HeaderIterator<'a> {
    fn new<'b>(conn: &'b raw::Connection) -> HeaderIterator<'b> {
        HeaderIterator { headers: get_headers(conn), position: 0 }
    }
}

impl<'a> Iterator<(&'a str, &'a str)> for HeaderIterator<'a> {
    fn next(&mut self) -> Option<(&'a str, &'a str)> {
        let pos = self.position;
        let headers = &self.headers;

        if self.headers.len() <= pos {
            None
        } else {
            let header = headers.get(pos);
            self.position += 1;
            header.name().map(|name| (name, header.value().unwrap()))
        }
    }
}

pub type ServerHandler<S, R> = fn(&mut Request) -> IoResult<Response<S, R>>;

#[allow(dead_code)]
pub struct Server(raw::Server);

impl Server {
    pub fn start<S: ToStatusCode, R: Reader + Send>(options: Config, handler: ServerHandler<S, R>) -> IoResult<Server> {
        fn internal_handler<S: ToStatusCode, R: Reader + Send>(conn: &mut raw::Connection, callback: &ServerHandler<S, R>) -> Result<(), ()> {
            let mut connection = Connection::new(conn).unwrap();
            let response = (*callback)(&mut connection.request);
            let writer = &mut connection;

            fn err<W: Writer>(writer: &mut W) {
                let _ = writeln!(writer, "HTTP/1.1 500 Internal Server Error");
            }

            match response {
                Ok(Response { status, headers, mut body }) => {
                    let (code, string) = try!(status.to_status().map_err(|_| err(writer)));
                    try!(write!(writer, "HTTP/1.1 {} {}\r\n", code, string).map_err(|_| ()));

                    for (key, value) in headers.iter() {
                        try!(write!(writer, "{}: {}\r\n", key, value).map_err(|_| ()));
                    }

                    try!(write!(writer, "\r\n").map_err(|_| ()));
                    try!(util::copy(&mut body, writer).map_err(|_| ()));
                },

                Err(_) => return Err(err(writer))
            }

            Ok(())
        }

        let raw_callback = raw::ServerCallback::new(internal_handler, handler);
        Ok(Server(raw::Server::start(options, raw_callback)))
    }
}

fn write_bytes(connection: &raw::Connection, bytes: &[u8]) -> Result<(), String> {
    let ret = raw::write(connection, bytes);

    if ret == -1 {
        return Err("Couldn't write bytes to the connection".to_str())
    }

    Ok(())
}

fn request_info<'a>(connection: &'a raw::Connection) -> Result<RequestInfo<'a>, String> {
    match get_request_info(connection) {
        Some(info) => Ok(info),
        None => Err("Couldn't get request info for connection".to_str())
    }
}

