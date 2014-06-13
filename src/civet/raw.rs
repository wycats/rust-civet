use std;
use native;
use std::mem::transmute;
use std::ptr::null;
use std::c_str::CString;
use libc;
use libc::{c_void,c_char,c_int,c_long,size_t};

#[link(name="civetweb")]
extern {
    fn mg_start(callbacks: *MgCallbacks, user_data: *c_void, options: **c_char) -> *MgContext;
    fn mg_stop(context: *MgContext);
    fn mg_set_request_handler(context: *MgContext, uri: *c_char, handler: MgRequestHandler, data: *c_void);
    fn mg_read(connection: *MgConnection, buf: *c_void, len: size_t) -> c_int;
    fn mg_write(connection: *MgConnection, data: *c_void, len: size_t) -> c_int;
    fn mg_get_header(connection: *MgConnection, name: *c_char) -> *c_char;
    fn mg_get_request_info(connection: *MgConnection) -> *MgRequestInfo;
}

pub enum MgConnection {}

enum MgContext {}

pub struct Context(*MgContext);

impl Context {
    fn unwrap(&self) -> *MgContext {
        match *self { Context(context) => context }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        debug!("Dropping context");
        match *self {
            Context(context) => unsafe { mg_stop(context) }
        }
    }
}

pub type MgRequestHandler = fn(*MgConnection, *c_void) -> int;

#[allow(dead_code)]
pub struct MgHeader {
    name: *c_char,
    value: *c_char
}

impl MgHeader {
    pub fn name(&self) -> Option<String> {
        to_str(self.name)
    }

    pub fn value(&self) -> Option<String> {
        to_str(self.value)
    }
}

pub struct RequestInfo<'a>(*MgRequestInfo);

impl<'a> RequestInfo<'a> {
    fn unwrap(&self) -> *MgRequestInfo {
        match *self { RequestInfo(info) => info }
    }
}

pub struct MgRequestInfo {
    request_method: *c_char,
    uri: *c_char,
    http_version: *c_char,
    query_string: *c_char,
    remote_user: *c_char,
    remote_ip: c_long,
    remote_port: c_int,
    is_ssl: bool,
    user_data: *c_void,
    conn_data: *c_void,

    num_headers: c_int,
    headers: [MgHeader, ..64]
}

#[allow(dead_code)]
pub struct MgCallbacks {
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
    pub fn new() -> MgCallbacks {
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

pub fn write_bytes(raw_request: &mut RawRequest, bytes: &[u8]) -> Result<(), String> {
    let c_bytes = bytes.as_ptr() as *c_void;
    let ret = unsafe { mg_write(raw_request.conn, c_bytes, bytes.len() as u64) };

    if ret == -1 {
        return Err("Couldn't write bytes to the connection".to_str())
    }

    Ok(())
}

pub fn get_header<'a>(raw_request: &'a RawRequest, string: &str) -> Option<String> {
    let cstr = unsafe { mg_get_header(raw_request.conn, string.to_c_str().unwrap()).to_option() };

    cstr.map(|c| unsafe { CString::new(c, false) }.as_str().to_str())
}

pub fn to_str(string: *c_char) -> Option<String> {
    unsafe {
        match string.to_option() {
            None => None,
            Some(c) => {
                if *string == 0 {
                    return None;
                }

                match CString::new(c, false).as_str() {
                    Some(s) => Some(s.to_str()),
                    None => None
                }
            }
        }
    }
}

pub fn get_headers(raw_request: &RawRequest) -> Result<[MgHeader, ..64], String> {
    let request_info = raw_request.request_info;

    if request_info.is_null() {
        Err("Couldn't get request info for connection".to_str())
    } else {
        let info = unsafe { *request_info };
        Ok(info.headers)
    }
}

pub fn read(raw_request: &mut RawRequest, buf: &mut [u8]) -> uint {
    (unsafe { mg_read(raw_request.conn, buf.as_ptr() as *c_void, buf.len() as u64) }) as uint
}

struct RawRequest {
    conn: *MgConnection,
    request_info: *MgRequestInfo
}

impl RawRequest {
    fn info<'a>(&'a self) -> &'a MgRequestInfo {
        unsafe { &*self.request_info }
    }

    pub fn request_method(&self) -> Option<String> {
        to_str(self.info().request_method)
    }

    pub fn uri(&self) -> Option<String> {
        to_str(self.info().uri)
    }

    pub fn http_version(&self) -> Option<String> {
        to_str(self.info().http_version)
    }

    pub fn query_string(&self) -> Option<String> {
        to_str(self.info().query_string)
    }

    pub fn remote_user(&self) -> Option<String> {
        to_str(self.info().remote_user)
    }

    pub fn remote_ip(&self) -> int {
        self.info().remote_ip as int
    }

    pub fn is_ssl(&self) -> bool {
        self.info().is_ssl
    }
}

pub fn start(handler: |&mut RawRequest|: 'static, options: **c_char) -> Context {
    fn internal_handler(conn: *MgConnection, handler: *c_void) -> int {
        let raw_request = match request_info(unsafe { transmute(conn) }) {
            Ok(info) => RawRequest { conn: conn, request_info: info.unwrap() },
            Err(err) => return 0
        };

        let (tx, rx) = channel();
        let handler: |&mut RawRequest|: 'static = unsafe { transmute(handler) };
        let mut task = native::task::new((0, std::uint::MAX));

        task.death.on_exit = Some(proc(r) tx.send(r));
        task.run(|| {
            println!("Made it so far");
            let _ = handler(&mut raw_request);
            println!("Done");
        });
        let _ = rx.recv();

        1
    }

    let context = unsafe { mg_start(&MgCallbacks::new(), null(), options) };
    unsafe { mg_set_request_handler(context, "**".to_c_str().unwrap(), internal_handler, transmute(handler)) };

    Context(context)
}

pub fn set_handler(context: &mut Context, handler: fn(*MgConnection, *c_void) -> int, param: *c_void) {
    unsafe { mg_set_request_handler(context.unwrap(), "**".to_c_str().unwrap(), handler, param) }
}

pub fn headers_len<'a>(raw_request: &'a RawRequest) -> Result<uint, String> {
    let info = try!(request_info(raw_request)).unwrap();
    Ok(unsafe { *info }.num_headers as uint)
}

pub fn request_info<'a>(raw_request: &'a RawRequest) -> Result<RequestInfo<'a>, String> {
    let request_info = unsafe { mg_get_request_info(raw_request.conn) };

    if request_info.is_null() {
        Err("Couldn't get request info for connection".to_str())
    } else {
        Ok(RequestInfo(request_info))
    }
}

pub trait WithCStrs {
    fn with_c_strs(&self, null_terminated: bool, f: |**libc::c_char|) ;
}

impl<'a, T: ToCStr> WithCStrs for &'a [T] {
    fn with_c_strs(&self, null_terminate: bool, f: |**c_char|) {
        let c_strs: Vec<CString> = self.iter().map(|s: &T| s.to_c_str()).collect();
        let mut ptrs: Vec<*c_char> = c_strs.iter().map(|c: &CString| c.with_ref(|ptr| ptr)).collect();
        if null_terminate {
            ptrs.push(null());
        }
        f(ptrs.as_ptr())
    }
}
