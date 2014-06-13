use std;
use std::mem::transmute;
use std::io;
use std::io::IoResult;
use libc::{c_void,c_char};
use native;
use civet;
use civet::raw::{RawRequest,RequestInfo,Context};
use civet::raw::{get_headers,get_header,headers_len,write_bytes,request_info};
use civet::raw::WithCStrs;

mod raw;

pub struct Config {
    pub port: uint,
    pub threads: uint
}

impl Config {
    pub fn default() -> Config {
        Config { port: 8888, threads: 50 }
    }
}

pub struct Connection<'a> {
    request: Request<'a>,
    response: Response<'a>
}

pub struct Request<'a> {
    conn: &'a RawRequest
}

impl<'a> Request<'a> {
    pub fn get_header<S: Str>(&self, string: S) -> Option<String> {
        get_header(self.conn, string.as_slice())
    }

    pub fn count_headers(&self) -> Result<uint, String> {
        headers_len(self.conn)
    }

    pub fn method(&self) -> Option<String> {
        self.conn.request_method()
    }

    pub fn url(&self) -> Option<String> {
        self.conn.uri()
    }

    pub fn http_version(&self) -> Option<String> {
        self.conn.http_version()
    }

    pub fn query_string(&self) -> Option<String> {
        self.conn.query_string()
    }

    pub fn remote_user(&self) -> Option<String> {
        self.conn.remote_user()
    }

    pub fn remote_ip(&self) -> int {
        self.conn.remote_ip() as int
    }

    pub fn is_ssl(&self) -> bool {
        self.conn.is_ssl()
    }

    pub fn headers<'a>(&'a self) -> Headers<'a> {
        Headers { conn: self.conn }
    }
}

pub struct Response<'a> {
    conn: &'a RawRequest
}

impl<'a> Connection<'a> {
    pub fn new<'a>(conn: &'a RawRequest) -> Connection<'a> {
        match request_info(conn) {
            Ok(info) => {
                let request = Request::<'a> { conn: conn };
                let response = Response::<'a> { conn: conn };
                Connection::<'a> {
                    request: request,
                    response: response
                }
            },
            Err(err) => fail!(err)
        }
    }

}

impl<'a> Writer for Response<'a> {
    fn write(&mut self, buf: &[u8]) -> IoResult<()> {
        write_bytes(self.conn, buf).map_err(|_| {
            io::standard_error(io::IoUnavailable)
        })
    }
}

impl<'a> Reader for Request<'a> {
    fn read(&mut self, buf: &mut[u8]) -> IoResult<uint> {
        let ret = civet::raw::read(self.conn, buf);

        if ret == 0 {
            Err(io::standard_error(io::EndOfFile))
        } else {
            Ok(ret as uint)
        }
    }
}

pub struct Headers<'a> {
    conn: &'a RawRequest
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
    conn: &'a RawRequest,
    position: uint
}

impl<'a> HeaderIterator<'a> {
    fn new<'a>(conn: &'a RawRequest) -> HeaderIterator<'a> {
        HeaderIterator { conn: conn, position: 0 }
    }
}

impl<'a> Iterator<(String, String)> for HeaderIterator<'a> {
    fn next(&mut self) -> Option<(String, String)> {
        let pos = self.position;

        match get_headers(self.conn).ok() {
            Some(headers) => {
                let header = headers[pos];

                header.name().map(|name| {
                    self.position += 1;
                    (name, header.value().unwrap())
                })
            },
            None => None
        }
    }
}

#[allow(dead_code)]
pub struct Server {
    context: Context,
}

impl Server {
    pub fn start(options: Config, handler: fn(&mut Request, &mut Response) -> IoResult<()>) -> IoResult<Server> {
        let Config { port, threads } = options;
        let options = ["listening_ports".to_str(), port.to_str(), "num_threads".to_str(), threads.to_str()];

        let mut server = None;

        debug!("Starting server");
        options.with_c_strs(true, |options: **c_char| {
            let mut context = civet::raw::start(|raw_request| {
                let connection = Connection::new(raw_request);
                handler(&mut connection.request, &mut connection.response);
            }, options);

            server = Some(Server { context: context });
        });
        debug!("Done starting server");

        Ok(server.unwrap())
    }
}


