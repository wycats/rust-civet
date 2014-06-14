use libc::{c_void,c_char,c_int,c_long,size_t};
use std::ptr::null;

#[link(name="civetweb")]
extern {
    fn mg_start(callbacks: *MgCallbacks, user_data: *c_void, options: **c_char) -> *MgContext;
    pub fn mg_set_request_handler(context: *MgContext, uri: *c_char, handler: MgRequestHandler, data: *c_void);
    pub fn mg_read(connection: *MgConnection, buf: *c_void, len: size_t) -> c_int;
    pub fn mg_write(connection: *MgConnection, data: *c_void, len: size_t) -> c_int;
    pub fn mg_get_header(connection: *MgConnection, name: *c_char) -> *c_char;
    pub fn mg_get_request_info(connection: *MgConnection) -> *MgRequestInfo;
}

pub enum MgContext {}
pub enum MgConnection {}

pub type MgRequestHandler = fn(*MgConnection, *c_void) -> int;

#[allow(dead_code)]
pub struct MgHeader {
    pub name: *c_char,
    pub value: *c_char
}

#[allow(dead_code)]
pub struct MgRequestInfo {
    pub request_method: *c_char,
    pub uri: *c_char,
    pub http_version: *c_char,
    pub query_string: *c_char,
    pub remote_user: *c_char,
    pub remote_ip: c_long,
    pub remote_port: c_int,
    pub is_ssl: bool,

    user_data: *c_void,
    conn_data: *c_void,

    pub num_headers: c_int,
    pub headers: [MgHeader, ..64]
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

pub fn start(handler: *c_void, options: **c_char) -> *MgContext {
    unsafe { mg_start(&MgCallbacks::new(), handler, options) }
}

pub fn read(conn: *MgConnection, buf: &mut [u8]) -> i32 {
    unsafe { mg_read(conn, buf.as_ptr() as *c_void, buf.len() as u64) }
}
