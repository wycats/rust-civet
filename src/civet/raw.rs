use libc::{c_void,c_char,c_int,c_long,size_t};
use std;
use std::c_str::CString;
use std::ptr::null;
use std::mem::transmute;
use native;

pub struct Config {
    pub port: uint,
    pub threads: uint
}

impl Config {
    pub fn default() -> Config {
        Config { port: 8888, threads: 50 }
    }
}

#[link(name="civetweb")]
extern {
    fn mg_start(callbacks: *MgCallbacks, user_data: *c_void, options: **c_char) -> *MgContext;
    fn mg_stop(context: *MgContext);
    fn mg_set_request_handler(context: *MgContext, uri: *c_char, handler: MgRequestHandler, data: *c_void);
    fn mg_read(connection: *mut MgConnection, buf: *c_void, len: size_t) -> c_int;
    fn mg_write(connection: *mut MgConnection, data: *c_void, len: size_t) -> c_int;
    fn mg_get_header(connection: *mut MgConnection, name: *c_char) -> *c_char;
    fn mg_get_request_info(connection: *mut MgConnection) -> *MgRequestInfo;
}

enum MgContext {}

pub struct Server(*MgContext);

pub struct ServerCallback<T> {
    callback: fn(&mut Connection, &T) -> Result<(), ()>,
    param: T
}

impl<T> ServerCallback<T> {
    pub fn new(callback: fn(&mut Connection, &T) -> Result<(), ()>, param: T) -> ServerCallback<T> {
        ServerCallback { callback: callback, param: param }
    }
}

impl Server {
    fn as_ref<'a>(&'a self) -> &'a MgContext {
        match *self { Server(context) => unsafe { &*context } }
    }

    pub fn start<T>(options: Config, callback: ServerCallback<T>) -> Server {
        let Config { port, threads } = options;
        let options = ["listening_ports".to_str(), port.to_str(), "num_threads".to_str(), threads.to_str()];

        let mut server = None;
        let mut cb = Some(box callback);

        options.with_c_strs(true, |options| {
            let context = start(options);
            server = Some(Server(context));

            unsafe { mg_set_request_handler(context, "**".to_c_str().unwrap(), raw_handler::<T>, transmute(cb.take_unwrap())) }
        });

        server.unwrap()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        unsafe { mg_stop(self.as_ref()) }
    }
}

fn raw_handler<T>(conn: *mut MgConnection, param: *c_void) -> int {
    let (tx, rx) = channel();
    let callback: Box<ServerCallback<T>> = unsafe { transmute(param) };

    let mut task = native::task::new((0, std::uint::MAX));
    task.death.on_exit = Some(proc(r) tx.send(r));

    let mut result = None;

    task.run(|| {
        let mut connection = Connection(conn);
        result = Some((callback.callback)(&mut connection, &callback.param))
    });
    let _ = rx.recv();

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

#[allow(dead_code)]
struct MgHeader {
    name: *c_char,
    value: *c_char
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

struct MgRequestInfo {
    request_method: *c_char,
    uri: *c_char,
    http_version: *c_char,
    query_string: *c_char,
    remote_user: *c_char,
    remote_ip: c_long,
    remote_port: c_int,
    is_ssl: bool,

    #[allow(dead_code)]
    user_data: *c_void,
    #[allow(dead_code)]
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

    pub fn remote_port(&self) -> int {
        self.as_ref().remote_port as int
    }

    pub fn is_ssl(&self) -> bool {
        self.as_ref().is_ssl
    }
}

#[allow(dead_code)]
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

pub fn start(options: **c_char) -> *MgContext {
    unsafe { mg_start(&MgCallbacks::new(), null(), options) }
}

pub fn read(conn: &Connection, buf: &mut [u8]) -> i32 {
    unsafe { mg_read(conn.unwrap(), buf.as_ptr() as *c_void, buf.len() as u64) }
}

pub fn write(conn: &Connection, bytes: &[u8]) -> i32 {
    let c_bytes = bytes.as_ptr() as *c_void;
    unsafe { mg_write(conn.unwrap(), c_bytes, bytes.len() as u64) }
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

trait WithCStrs {
    fn with_c_strs(&self, null_terminated: bool, f: |**c_char|) ;
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
