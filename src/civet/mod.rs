use std;
use std::mem::transmute;
use std::io;
use std::io::IoResult;
use libc::{c_void,c_char};
use native;
use civet;
use civet::raw::{MgConnection,MgRequestInfo,Context};
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
    conn: &'a MgConnection,
    request_info: &'a MgRequestInfo
}

impl<'a> Request<'a> {
    pub fn get_header<S: Str>(&self, string: S) -> Option<String> {
        get_header(self.conn, string.as_slice())
    }

    pub fn count_headers(&self) -> Result<uint, String> {
        headers_len(self.conn)
    }

    pub fn method(&self) -> Option<String> {
        self.request_info.request_method()
    }

    pub fn url(&self) -> Option<String> {
        self.request_info.uri()
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
        self.request_info.remote_ip as int
    }

    pub fn is_ssl(&self) -> bool {
        self.request_info.is_ssl
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
    conn: &'a MgConnection,
    position: uint
}

impl<'a> HeaderIterator<'a> {
    fn new<'a>(conn: &'a MgConnection) -> HeaderIterator<'a> {
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

        fn internal_handler(conn: *MgConnection, handler: *c_void) -> int {
            let _ = Connection::new(unsafe { conn.to_option() }.unwrap()).map(|mut connection| {
                let (tx, rx) = channel();
                let handler: fn(&mut Request, &mut Response) -> IoResult<()> = unsafe { transmute(handler) };
                let mut task = native::task::new((0, std::uint::MAX));

                task.death.on_exit = Some(proc(r) tx.send(r));
                task.run(|| {
                    println!("Made it so far");
                    let _ = handler(&mut connection.request, &mut connection.response);
                    println!("Done");
                });
                let _ = rx.recv();
            });

            1
        }

        let mut server = None;

        debug!("Starting server");
        options.with_c_strs(true, |options: **c_char| {
            let mut context = civet::raw::start(unsafe { transmute(handler) }, options);
            civet::raw::set_handler(&mut context, internal_handler, unsafe { transmute(handler) });
            server = Some(Server { context: context });
        });
        debug!("Done starting server");

        Ok(server.unwrap())
    }
}


