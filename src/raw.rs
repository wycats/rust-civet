use libc::{c_void,c_char,c_int,c_long,size_t};
use native;
use std::c_str::CString;
use std::mem::transmute;
use std::ptr::null;
use std;

pub struct Config {
    pub port: uint,
    pub threads: uint
}

impl Config {
    pub fn default() -> Config {
        Config { port: 8888, threads: 50 }
    }
}

#[link(name = "civetweb", kind = "static")]
extern {
    fn mg_start(callbacks: *MgCallbacks, user_data: *c_void,
                options: **c_char) -> *MgContext;
    fn mg_stop(context: *MgContext);
    fn mg_set_request_handler(context: *MgContext, uri: *c_char,
                              handler: MgRequestHandler, data: *c_void);
    fn mg_read(connection: *mut MgConnection, buf: *c_void,
               len: size_t) -> c_int;
    fn mg_write(connection: *mut MgConnection, data: *c_void,
                len: size_t) -> c_int;
    fn mg_get_header(connection: *mut MgConnection, name: *c_char) -> *c_char;
    fn mg_get_request_info(connection: *mut MgConnection) -> *MgRequestInfo;
}

enum MgContext {}

pub struct Server<T>(*MgContext, Box<ServerCallback<T>>);

pub struct ServerCallback<T> {
    callback: fn(&mut Connection, &T) -> Result<(), ()>,
    param: T
}

impl<T: Share> ServerCallback<T> {
    pub fn new(callback: fn(&mut Connection, &T) -> Result<(), ()>,
               param: T) -> ServerCallback<T> {
        ServerCallback { callback: callback, param: param }
    }
}

impl<T: 'static + Share> Server<T> {
    fn as_ref<'a>(&'a self) -> &'a MgContext {
        let Server(context, _) = *self;
        unsafe { &*context }
    }

    pub fn start(options: Config, callback: ServerCallback<T>) -> Server<T> {
        let Config { port, threads } = options;
        let options = vec!(
            "listening_ports".to_c_str(), port.to_str().to_c_str(),
            "num_threads".to_c_str(), threads.to_str().to_c_str(),
        );
        let mut ptrs: Vec<_> = options.iter().map(|a| a.with_ref(|p| p)).collect();
        ptrs.push(0 as *_);

        let context = start(ptrs.as_ptr());
        let uri = "**".to_c_str();
        let callback = box callback;
        unsafe {
            mg_set_request_handler(context, uri.with_ref(|p| p),
                                   raw_handler::<T>,
                                   &*callback as *_ as *c_void);
        }
        Server(context, callback)
    }
}

#[unsafe_destructor]
impl<T: 'static + Share> Drop for Server<T> {
    fn drop(&mut self) {
        unsafe { mg_stop(self.as_ref()) }
    }
}

fn raw_handler<T: 'static>(conn: *mut MgConnection, param: *c_void) -> int {
    let callback: &ServerCallback<T> = unsafe { transmute(param) };
    let task = native::task::new((0, std::uint::MAX));
    let mut result = None;

    task.run(|| {
        let mut connection = Connection(conn);
        result = Some((callback.callback)(&mut connection, &callback.param));
    });

    match result {
        None => 0,
        Some(Err(_)) => 0,
        Some(Ok(_)) => 1
    }
}

pub enum MgConnection {}

pub struct Connection(*mut MgConnection);

impl Connection {
    fn unwrap(&self) -> *mut MgConnection {
        match *self { Connection(conn) => conn }
    }
}

type MgRequestHandler = fn(*mut MgConnection, *c_void) -> int;

#[repr(C)]
struct MgHeader {
    name: *c_char,
    value: *c_char
}

pub struct Header<'a>(*MgHeader);

impl<'a> Header<'a> {
    fn as_ref(&self) -> &'a MgHeader {
        match *self { Header(header) => unsafe { &*header } }
    }

    pub fn name(&self) -> Option<&'a str> {
        to_slice(self.as_ref(), |header| header.name)
    }

    pub fn value(&self) -> Option<&'a str> {
        to_slice(self.as_ref(), |header| header.value)
    }
}

#[repr(C)]
struct MgRequestInfo {
    request_method: *c_char,
    uri: *c_char,
    http_version: *c_char,
    query_string: *c_char,
    remote_user: *c_char,
    remote_ip: c_long,
    remote_port: c_int,
    is_ssl: c_int,

    user_data: *c_void,
    conn_data: *c_void,

    num_headers: c_int,
    headers: [MgHeader, ..64]
}

pub struct RequestInfo<'a>(*MgRequestInfo);

impl<'a> RequestInfo<'a> {
    pub fn as_ref<'a>(&'a self) -> &'a MgRequestInfo {
        match *self { RequestInfo(info) => unsafe { &*info } }
    }

    pub fn num_headers(&self) -> int {
        self.as_ref().num_headers as int
    }

    pub fn method<'a>(&'a self) -> Option<&'a str> {
        to_slice(self.as_ref(), |info| info.request_method)
    }

    pub fn url<'a>(&'a self) -> Option<&'a str> {
        to_slice(self.as_ref(), |info| info.uri)
    }

    pub fn http_version<'a>(&'a self) -> Option<&'a str> {
        to_slice(self.as_ref(), |info| info.http_version)
    }

    pub fn query_string<'a>(&'a self) -> Option<&'a str> {
        to_slice(self.as_ref(), |info| info.query_string)
    }

    pub fn remote_user<'a>(&'a self) -> Option<&'a str> {
        to_slice(self.as_ref(), |info| info.remote_user)
    }

    pub fn remote_ip(&self) -> int {
        self.as_ref().remote_ip as int
    }

    pub fn remote_port(&self) -> int {
        self.as_ref().remote_port as int
    }

    pub fn is_ssl(&self) -> bool {
        self.as_ref().is_ssl != 0
    }
}

#[repr(C)]
struct MgCallbacks {
    begin_request: *c_void,
    end_request: *c_void,
    log_message: *c_void,
    init_ssl: *c_void,
    websocket_connect: *c_void,
    websocket_ready: *c_void,
    websocket_data: *c_void,
    connection_close: *c_void,
    open_file: *c_void,
    init_lua: *c_void,
    upload: *c_void,
    http_error: *c_void
}

impl MgCallbacks {
    fn new() -> MgCallbacks {
        MgCallbacks {
            begin_request: null(),
            end_request: null(),
            log_message: null(),
            init_ssl: null(),
            websocket_connect: null(),
            websocket_ready: null(),
            websocket_data: null(),
            connection_close: null(),
            open_file: null(),
            init_lua: null(),
            upload: null(),
            http_error: null()
        }
    }
}

fn to_slice<'a, T>(obj: &'a T, callback: |&'a T|:'static -> *c_char) -> Option<&'a str> {
    let chars = callback(obj);

    if unsafe { chars.is_null() || *chars == 0 } {
        return None;
    }

    let c_string = unsafe { CString::new(chars, false) };
    let len = c_string.len();

    unsafe { Some(transmute(std::raw::Slice { data: chars, len: len })) }
}

pub fn start(options: **c_char) -> *MgContext {
    unsafe { mg_start(&MgCallbacks::new(), null(), options) }
}

pub fn read(conn: &Connection, buf: &mut [u8]) -> i32 {
    unsafe { mg_read(conn.unwrap(), buf.as_ptr() as *c_void, buf.len() as size_t) }
}

pub fn write(conn: &Connection, bytes: &[u8]) -> i32 {
    let c_bytes = bytes.as_ptr() as *c_void;
    unsafe { mg_write(conn.unwrap(), c_bytes, bytes.len() as size_t) }
}

pub fn get_header(conn: &Connection, string: &str) -> Option<String> {
    let cstr = unsafe { mg_get_header(conn.unwrap(), string.to_c_str().unwrap()).to_option() };

    cstr.map(|c| unsafe { CString::new(c, false) }.as_str().to_str())
}

pub fn get_request_info<'a>(conn: &'a Connection) -> Option<RequestInfo<'a>> {
    (unsafe { mg_get_request_info(conn.unwrap()).to_option() }).map(|info| RequestInfo(info))
}

pub fn get_headers<'a>(conn: &'a Connection) -> Vec<Header<'a>> {
    match get_request_info(conn) {
        Some(info) => info.as_ref().headers.iter().map(|h| Header(h)).collect(),
        None => vec!()
    }
}
