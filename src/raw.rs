use libc::{c_void,c_char,c_int,c_long,size_t};
use std::c_str::CString;
use std::io;
use std::mem::transmute;
use std::ptr::{null, null_mut};
use std;
use rustrt::task::Task;

pub struct Config {
    pub port: u16,
    pub threads: uint
}

impl Config {
    pub fn default() -> Config {
        Config { port: 8888, threads: 50 }
    }
}

extern {
    fn mg_start(callbacks: *const MgCallbacks, user_data: *mut c_void,
                options: *const *mut c_char) -> *mut MgContext;
    fn mg_stop(context: *mut MgContext);
    fn mg_set_request_handler(context: *mut MgContext, uri: *const c_char,
                              handler: MgRequestHandler, data: *mut c_void);
    fn mg_read(connection: *mut MgConnection, buf: *mut c_void,
               len: size_t) -> c_int;
    fn mg_write(connection: *mut MgConnection, data: *const c_void,
                len: size_t) -> c_int;
    fn mg_get_header(connection: *mut MgConnection,
                     name: *const c_char) -> *const c_char;
    fn mg_get_request_info(connection: *mut MgConnection) -> *mut MgRequestInfo;
}

enum MgContext {}

pub struct Server<T>(*mut MgContext, Box<ServerCallback<T>>);

pub struct ServerCallback<T> {
    callback: fn(&mut Connection, &T) -> Result<(), ()>,
    param: T,
}

impl<T: Sync> ServerCallback<T> {
    pub fn new(callback: fn(&mut Connection, &T) -> Result<(), ()>,
               param: T) -> ServerCallback<T> {
        ServerCallback { callback: callback, param: param }
    }
}

impl<T: 'static + Sync> Server<T> {
    fn as_ptr(&self) -> *mut MgContext {
        let Server(context, _) = *self; context
    }

    pub fn start(options: Config,
                 callback: ServerCallback<T>) -> io::IoResult<Server<T>> {
        let Config { port, threads } = options;
        let options = vec!(
            "listening_ports".to_c_str(), port.to_string().to_c_str(),
            "num_threads".to_c_str(), threads.to_string().to_c_str(),
        );
        let mut ptrs: Vec<*const c_char> = options.iter().map(|a| {
            a.as_ptr()
        }).collect();
        ptrs.push(0 as *const c_char);

        let context = start(ptrs.as_ptr() as *const _);
        // TODO: fill in this error
        if context.is_null() { return Err(io::standard_error(io::OtherIoError)) }

        let uri = "**".to_c_str();
        let mut callback = box callback;
        unsafe {
            mg_set_request_handler(context, uri.as_ptr(),
                                   raw_handler::<T>,
                                   &mut *callback as *mut _ as *mut c_void);
        }
        Ok(Server(context, callback))
    }
}

#[unsafe_destructor]
impl<T: 'static + Sync> Drop for Server<T> {
    fn drop(&mut self) {
        unsafe { mg_stop(self.as_ptr()) }
    }
}

fn raw_handler<T: 'static>(conn: *mut MgConnection, param: *mut c_void) -> int {
    let callback: &ServerCallback<T> = unsafe { transmute(param) };
    let task = box Task::new(None, None);
    let mut result = None;

    task.run(|| {
        let mut connection = Connection(conn);
        result = Some((callback.callback)(&mut connection, &callback.param));
    }).destroy();

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

type MgRequestHandler = fn(*mut MgConnection, *mut c_void) -> int;

#[repr(C)]
struct MgHeader {
    name: *const c_char,
    value: *const c_char
}

pub struct Header<'a>(*mut MgHeader);

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
    request_method: *const c_char,
    uri: *const c_char,
    http_version: *const c_char,
    query_string: *const c_char,
    remote_user: *const c_char,
    remote_ip: c_long,
    remote_port: c_int,
    is_ssl: c_int,

    user_data: *mut c_void,
    conn_data: *mut c_void,

    num_headers: c_int,
    headers: [MgHeader, ..64]
}

pub struct RequestInfo<'a>(*mut MgRequestInfo);

impl<'a> RequestInfo<'a> {
    pub fn as_ref(&self) -> &MgRequestInfo {
        match *self { RequestInfo(info) => unsafe { &*info } }
    }

    fn as_ptr(&self) -> *mut MgRequestInfo {
        match *self { RequestInfo(info) => info }
    }

    pub fn method(&self) -> Option<&str> {
        to_slice(self.as_ref(), |info| info.request_method)
    }

    pub fn url(&self) -> Option<&str> {
        to_slice(self.as_ref(), |info| info.uri)
    }

    pub fn http_version(&self) -> Option<&str> {
        to_slice(self.as_ref(), |info| info.http_version)
    }

    pub fn query_string(&self) -> Option<&str> {
        to_slice(self.as_ref(), |info| info.query_string)
    }

    pub fn remote_ip(&self) -> int {
        self.as_ref().remote_ip as int
    }

    pub fn is_ssl(&self) -> bool {
        self.as_ref().is_ssl != 0
    }
}

#[repr(C)]
struct MgCallbacks {
    begin_request: *const c_void,
    end_request: *const c_void,
    log_message: *const c_void,
    init_ssl: *const c_void,
    websocket_connect: *const c_void,
    websocket_ready: *const c_void,
    websocket_data: *const c_void,
    connection_close: *const c_void,
    open_file: *const c_void,
    init_lua: *const c_void,
    upload: *const c_void,
    http_error: *const c_void
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

fn to_slice<'a, T>(obj: &'a T,
                   callback: |&'a T| -> *const c_char) -> Option<&'a str> {
    let chars = callback(obj);

    if unsafe { chars.is_null() || *chars == 0 } {
        return None;
    }

    let c_string = unsafe { CString::new(chars, false) };
    let len = c_string.len();

    unsafe { Some(transmute(std::raw::Slice { data: chars, len: len })) }
}

pub fn start(options: *const *mut c_char) -> *mut MgContext {
    unsafe { mg_start(&MgCallbacks::new(), null_mut(), options) }
}

pub fn read(conn: &Connection, buf: &mut [u8]) -> i32 {
    unsafe { mg_read(conn.unwrap(), buf.as_mut_ptr() as *mut c_void, buf.len() as size_t) }
}

pub fn write(conn: &Connection, bytes: &[u8]) -> i32 {
    let c_bytes = bytes.as_ptr() as *const c_void;
    unsafe { mg_write(conn.unwrap(), c_bytes, bytes.len() as size_t) }
}

pub fn get_header<'a>(conn: &'a Connection, string: &str) -> Option<&'a str> {
    let string = string.to_c_str();

    unsafe {
        to_slice(conn, |conn| mg_get_header(conn.unwrap(), string.as_ptr()))
    }
}

pub fn get_request_info<'a>(conn: &'a Connection) -> Option<RequestInfo<'a>> {
    unsafe {
        let info = mg_get_request_info(conn.unwrap());
        if info.is_null() {
            None
        } else {
            Some(RequestInfo(info))
        }
    }
}

pub fn get_headers<'a>(conn: &'a Connection) -> Vec<Header<'a>> {
    match get_request_info(conn) {
        Some(info) => unsafe {
            (*info.as_ptr()).headers.iter_mut().map(|h| Header(h)).collect()
        },
        None => vec!()
    }
}
