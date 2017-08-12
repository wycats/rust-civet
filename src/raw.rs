use libc::{c_void,c_char,c_int,c_longlong,size_t};
use std::ffi::{CStr, CString};
use std::io;
use std::marker;
use std::mem::transmute;
use std::ptr::{null, null_mut};
use std::panic;
use std::str;

use Config;

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

pub enum MgContext {}

pub struct Server<T: Sync + 'static>(*mut MgContext, Box<ServerCallback<T>>);

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
                 callback: ServerCallback<T>) -> io::Result<Server<T>> {
        let (_a, ptrs) = ::config::config_to_options(&options);

        let context = start(ptrs.as_ptr() as *const _);
        // TODO: fill in this error
        if context.is_null() {
            return Err(io::Error::new(io::ErrorKind::Other, "other error"))
        }

        let uri = CString::new("**").unwrap();
        let mut callback = Box::new(callback);
        unsafe {
            mg_set_request_handler(context, uri.as_ptr(),
                                   raw_handler::<T>,
                                   &mut *callback as *mut _ as *mut c_void);
        }
        Ok(Server(context, callback))
    }
}

impl<T: 'static + Sync> Drop for Server<T> {
    fn drop(&mut self) {
        unsafe { mg_stop(self.as_ptr()) }
    }
}

extern fn raw_handler<T: 'static>(conn: *mut MgConnection,
                                  param: *mut c_void) -> i32 {
    struct Env(*mut MgConnection, *mut c_void);
    unsafe impl Send for Env {}

    let env = Env(conn, param);
    let ret = panic::catch_unwind(move || {
        let Env(conn, param) = env;
        let callback: &ServerCallback<T> = unsafe { transmute(param) };

        let mut connection = Connection(conn);
        (callback.callback)(&mut connection, &callback.param)
    });

    match ret {
        Err(..) => 0,
        Ok(..) => 1,
    }
}

pub enum MgConnection {}

pub struct Connection(*mut MgConnection);

impl Connection {
    fn unwrap(&self) -> *mut MgConnection {
        match *self { Connection(conn) => conn }
    }
}

type MgRequestHandler = extern fn(*mut MgConnection, *mut c_void) -> i32;

#[repr(C)]
struct MgHeader {
    name: *const c_char,
    value: *const c_char
}

pub struct Header<'a> {
    ptr: *mut MgHeader,
    _marker: marker::PhantomData<&'a str>,
}

impl<'a> Header<'a> {
    fn as_ref(&self) -> &'a MgHeader {
        unsafe { &*self.ptr }
    }

    pub fn name(&self) -> Option<&'a str> {
        to_slice(self.as_ref(), |header| header.name)
    }

    pub fn value(&self) -> Option<&'a str> {
        to_slice(self.as_ref(), |header| header.value)
    }
}

#[repr(C)]
pub struct MgRequestInfo {
    request_method: *const c_char,
    request_uri: *const c_char,
    local_uri: *const c_char,
    uri: *const c_char,
    http_version: *const c_char,
    query_string: *const c_char,
    remote_user: *const c_char,
    remote_addr: [c_char; 48],
    content_length: c_longlong,
    remote_port: c_int,
    is_ssl: c_int,

    user_data: *mut c_void,
    conn_data: *mut c_void,

    num_headers: c_int,
    headers: [MgHeader; 64],

    client_cert: *mut c_void,
    accepted_websocket_subprotocol: *const c_char
}

pub struct RequestInfo<'a> {
    ptr: *mut MgRequestInfo,
    _marker: marker::PhantomData<&'a str>,
}

impl<'a> RequestInfo<'a> {
    pub fn as_ref(&self) -> &MgRequestInfo {
        unsafe { &*self.as_ptr() }
    }

    fn as_ptr(&self) -> *mut MgRequestInfo {
        self.ptr
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

    pub fn remote_addr(&self) -> &str {
        unsafe {
            CStr::from_ptr(
                self.as_ref().remote_addr.as_ptr()
            ).to_str().unwrap_or("0.0.0.0")
        }
    }

    pub fn remote_port(&self) -> u16 {
        self.as_ref().remote_port as u16
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
    log_access: *const c_void,
    init_ssl: *const c_void,
    connection_close: *const c_void,
    open_file: *const c_void,
    init_lua: *const c_void,
    http_error: *const c_void,
    init_context: *const c_void,
    init_thread: *const c_void,
    exit_context: *const c_void
}

impl MgCallbacks {
    fn new() -> MgCallbacks {
        MgCallbacks {
            begin_request: null(),
            end_request: null(),
            log_message: null(),
            log_access: null(),
            init_ssl: null(),
            connection_close: null(),
            open_file: null(),
            init_lua: null(),
            http_error: null(),
            init_context: null(),
            init_thread: null(),
            exit_context: null()
        }
    }
}

fn to_slice<'a, T, F>(obj: &'a T, mut callback: F) -> Option<&'a str>
    where F: FnMut(&'a T) -> *const c_char
{
    let chars = callback(obj);

    if unsafe { chars.is_null() || *chars == 0 } {
        return None;
    }

    Some(str::from_utf8(unsafe { CStr::from_ptr(chars).to_bytes() }).unwrap())
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
    let string = CString::new(string).unwrap();

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
            Some(RequestInfo { ptr: info, _marker: marker::PhantomData })
        }
    }
}

pub fn get_headers<'a>(conn: &'a Connection) -> Vec<Header<'a>> {
    match get_request_info(conn) {
        Some(info) => unsafe {
            (*info.as_ptr()).headers.iter_mut().map(|h| {
                Header { ptr: h, _marker: marker::PhantomData }
            }).collect()
        },
        None => vec!()
    }
}
