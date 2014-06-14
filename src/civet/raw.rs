use libc::{c_void,c_char,c_int,c_long,size_t};
use std::c_str::CString;
use std::ptr::null;

#[link(name="civetweb")]
extern {
    fn mg_start(callbacks: *MgCallbacks, user_data: *c_void, options: **c_char) -> *MgContext;
    pub fn mg_set_request_handler(context: *MgContext, uri: *c_char, handler: MgRequestHandler, data: *c_void);
    fn mg_read(connection: *MgConnection, buf: *c_void, len: size_t) -> c_int;
    fn mg_write(connection: *MgConnection, data: *c_void, len: size_t) -> c_int;
    fn mg_get_header(connection: *MgConnection, name: *c_char) -> *c_char;
    fn mg_get_request_info(connection: *MgConnection) -> *MgRequestInfo;
}

pub enum MgContext {}
pub enum MgConnection {}

pub type MgRequestHandler = fn(*MgConnection, *c_void) -> int;

#[allow(dead_code)]
pub struct MgHeader {
    pub name: *c_char,
    pub value: *c_char
}

pub struct Header<'a>(*MgHeader);

impl<'a> Header<'a> {
    fn as_ref<'a>(&'a self) -> &'a MgHeader {
        match *self { Header(header) => unsafe { &*header } }
    }

    pub fn name(&self) -> Option<String> {
        to_str(self.as_ref().name)
    }

    pub fn value(&self) -> Option<String> {
        to_str(self.as_ref().value)
    }
}

pub struct MgRequestInfo {
    pub request_method: *c_char,
    pub uri: *c_char,
    pub http_version: *c_char,
    pub query_string: *c_char,
    pub remote_user: *c_char,
    pub remote_ip: c_long,
    pub remote_port: c_int,
    pub is_ssl: bool,

    #[allow(dead_code)]
    user_data: *c_void,
    #[allow(dead_code)]
    conn_data: *c_void,

    pub num_headers: c_int,
    pub headers: [MgHeader, ..64]
}

pub struct RequestInfo<'a>(*MgRequestInfo);

impl<'a> RequestInfo<'a> {
    pub fn as_ref<'a>(&'a self) -> &'a MgRequestInfo {
        match *self { RequestInfo(info) => unsafe { &*info } }
    }

    pub fn method(&self) -> Option<String> {
        to_str(self.as_ref().request_method)
    }

    pub fn url(&self) -> Option<String> {
        to_str(self.as_ref().uri)
    }

    pub fn http_version(&self) -> Option<String> {
        to_str(self.as_ref().http_version)
    }

    pub fn query_string(&self) -> Option<String> {
        to_str(self.as_ref().query_string)
    }

    pub fn remote_user(&self) -> Option<String> {
        to_str(self.as_ref().remote_user)
    }

    pub fn remote_ip(&self) -> int {
        self.as_ref().remote_ip as int
    }

    pub fn is_ssl(&self) -> bool {
        self.as_ref().is_ssl
    }
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

fn to_str(string: *c_char) -> Option<String> {
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

pub fn start(handler: *c_void, options: **c_char) -> *MgContext {
    unsafe { mg_start(&MgCallbacks::new(), handler, options) }
}

pub fn read(conn: *MgConnection, buf: &mut [u8]) -> i32 {
    unsafe { mg_read(conn, buf.as_ptr() as *c_void, buf.len() as u64) }
}

pub fn write(conn: *MgConnection, bytes: &[u8]) -> i32 {
    let c_bytes = bytes.as_ptr() as *c_void;
    unsafe { mg_write(conn, c_bytes, bytes.len() as u64) }
}

pub fn get_header(conn: *MgConnection, string: &str) -> Option<String> {
    let cstr = unsafe { mg_get_header(conn, string.to_c_str().unwrap()).to_option() };

    cstr.map(|c| unsafe { CString::new(c, false) }.as_str().to_str())
}

pub fn get_request_info<'a>(conn: &'a MgConnection) -> Option<RequestInfo<'a>> {
    (unsafe { mg_get_request_info(conn).to_option() }).map(|info| RequestInfo(info))
}

pub fn get_headers<'a>(conn: &'a MgConnection) -> Vec<Header<'a>> {
    match get_request_info(conn) {
        Some(info) => info.as_ref().headers.iter().map(|h| Header(h)).collect(),
        None => vec!()
    }
}
