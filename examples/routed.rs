extern crate civet;
extern crate green;
extern crate rustuv;
extern crate routing = "route_recognizer";

use std::io::{IoResult, MemReader};
use std::collections::HashMap;

use civet::{Config, Server, Request, Response, Handler};
use routing::{Router, Params};

struct MyServer {
    router: Router<fn(&mut Request, &Params) -> IoResult<Response>>,
}

impl Handler for MyServer {
    fn call(&self, req: &mut Request) -> IoResult<Response> {
        let hit = match self.router.recognize(req.url().unwrap_or("")) {
            Ok(m) => m,
            Err(e) => fail!("{}", e),
        };
        (*hit.handler)(req, &hit.params)
    }
}

fn main() {
    let mut server = MyServer {
        router: Router::new(),
    };
    server.router.add("/:id", id);
    server.router.add("/", root);
    let _a = Server::start(Config { port: 8888, threads: 50 }, server);
    wait_for_sigint();
}

// libnative doesn't have signal handling yet
fn wait_for_sigint() {
    use std::io::signal::{Listener, Interrupt};
    use std::rt::task::TaskOpts;
    use green::{SchedPool, PoolConfig};

    let mut config = PoolConfig::new();
    config.event_loop_factory = rustuv::event_loop;

    let mut pool = SchedPool::new(config);
    pool.spawn(TaskOpts::new(), proc() {
        let mut l = Listener::new();
        l.register(Interrupt).unwrap();
        l.rx.recv();
    });
    pool.shutdown();
}

fn root(_req: &mut Request, _params: &Params) -> IoResult<Response> {
    let response = "you found the root!\n".as_bytes().to_owned();
    Ok(Response::new(200, HashMap::new(), MemReader::new(response)))
}

fn id(_req: &mut Request, params: &Params) -> IoResult<Response> {
    let response = format!("you found the id {}!\n", params["id"]);
    let response = response.into_bytes();
    Ok(Response::new(200, HashMap::new(), MemReader::new(response)))
}
