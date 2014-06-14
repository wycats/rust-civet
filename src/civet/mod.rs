use std::io;
use std::io::IoResult;

use civet;
use civet::raw::{MgConnection};
use civet::raw::{RequestInfo,Header};
use civet::raw::{get_header,get_headers,get_request_info};

pub use civet::raw::Config;

mod raw;

pub struct Connection<'a> {
    request: Request<'a>,
    response: Response<'a>
}

pub struct Request<'a> {
    conn: &'a MgConnection,
    request_info: RequestInfo<'a>
}

impl<'a> Request<'a> {
    pub fn get_header<S: Str>(&self, string: S) -> Option<String> {
        get_header(self.conn, string.as_slice())
    }

    pub fn count_headers(&self) -> uint {
        self.request_info.num_headers() as uint
    }

    pub fn method(&self) -> Option<String> {
        self.request_info.method()
    }

    pub fn url(&self) -> Option<String> {
        self.request_info.url()
    }

    pub fn http_version(&self) -> Option<String> {
        self.request_info.http_version()
    }

    pub fn query_string(&self) -> Option<String> {
        self.request_info.query_string()
    }

    pub fn remote_user(&self) -> Option<String> {
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

pub struct Response<'a> {
    conn: &'a MgConnection
}

impl<'a> Connection<'a> {
    pub fn new<'a>(conn: &'a MgConnection) -> Result<Connection<'a>, String> {
        match request_info(conn) {
            Ok(info) => {
                let request = Request { conn: conn, request_info: info };
                let response = Response { conn: conn };
                Ok(Connection {
                    request: request,
                    response: response
                })
            },
            Err(err) => Err(err)
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
    conn: &'a MgConnection
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
    fn new<'a>(conn: &'a MgConnection) -> HeaderIterator<'a> {
        HeaderIterator { headers: get_headers(conn), position: 0 }
    }
}

impl<'a> Iterator<(String, String)> for HeaderIterator<'a> {
    fn next(&mut self) -> Option<(String, String)> {
        let pos = self.position;
        let headers = &self.headers;

        if headers.len() <= pos {
            None
        } else {
            let header = headers.get(pos);
            self.position += 1;
            header.name().map(|name| (name, header.value().unwrap()))
        }
    }
}

type ServerHandler = fn(&mut Request, &mut Response) -> IoResult<()>;

#[allow(dead_code)]
pub struct Server(civet::raw::Server);

impl Server {
    pub fn start(options: Config, handler: ServerHandler) -> IoResult<Server> {
        fn internal_handler(conn: &mut MgConnection, callback: &ServerHandler) -> Result<(), ()> {
            let mut connection = Connection::new(conn).unwrap();
            (*callback)(&mut connection.request, &mut connection.response).map_err(|_| ())
        }

        let raw_callback = civet::raw::ServerCallback::new(internal_handler, handler);
        Ok(Server(civet::raw::Server::start(options, raw_callback)))
    }
}

fn write_bytes(connection: &MgConnection, bytes: &[u8]) -> Result<(), String> {
    let ret = civet::raw::write(connection, bytes);

    if ret == -1 {
        return Err("Couldn't write bytes to the connection".to_str())
    }

    Ok(())
}

fn request_info<'a>(connection: &'a MgConnection) -> Result<RequestInfo<'a>, String> {
    match get_request_info(connection) {
        Some(info) => Ok(info),
        None => Err("Couldn't get request info for connection".to_str())
    }
}

