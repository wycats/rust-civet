use libc::c_char;
use std::ffi::CString;

pub struct Config {
    port: Option<u16>,
    threads: Option<u32>,
    enable_keep_alive: Option<bool>,
}

impl Config {
    pub fn new() -> Config {
        Config {
            port: None,
            threads: None,
            enable_keep_alive: None,
        }
    }

    pub fn port(&mut self, port: u16) -> &mut Config {
        self.port = Some(port);
        self
    }

    pub fn threads(&mut self, threads: u32) -> &mut Config {
        self.threads = Some(threads);
        self
    }

    pub fn keep_alive(&mut self, keep_alive: bool) -> &mut Config {
        self.enable_keep_alive = Some(keep_alive);
        self
    }
}

pub fn config_to_options(config: &Config) -> (Vec<CString>, Vec<*const c_char>) {
    let Config { port, threads, enable_keep_alive } = *config;
    let mut options = Vec::new();
    opt(&mut options, "listening_ports", port.map(|i| i.to_string()));
    opt(&mut options, "num_threads", threads.map(|i| i.to_string()));
    opt(&mut options, "enable_keep_alive", enable_keep_alive.map(|b| {
        (if b {"yes"} else {"no"}).to_string()
    }));
    let mut ptrs: Vec<*const c_char> = options.iter().map(|a| {
        a.as_ptr()
    }).collect();
    ptrs.push(0 as *const c_char);
    return (options, ptrs);

    fn opt(v: &mut Vec<CString>, name: &str, opt: Option<String>) {
        if let Some(t) = opt {
            v.push(CString::new(name).unwrap());
            v.push(CString::new(t).unwrap());
        }
    }
}
