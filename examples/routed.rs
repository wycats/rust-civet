extern crate conduit;
extern crate civet;
extern crate green;
extern crate rustuv;
extern crate routing = "route_recognizer";

use std::io::{IoResult, MemReader};
use std::collections::HashMap;
use std::fmt::Show;

use civet::{Config, Server, response};
use conduit::{Request, Response};
use routing::{Router, Params};

struct MyServer {
    router: Router<fn(&mut Request, &Params) -> IoResult<Response>>,
}

impl conduit::Handler for MyServer {
    fn call(&self, req: &mut Request) -> Result<Response, Box<Show>> {
        let hit = match self.router.recognize(req.path()) {
            Ok(m) => m,
            Err(e) => fail!("{}", e),
        };
        (*hit.handler)(req, &hit.params).map_err(|e| box e as Box<Show>)
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
    use green::{SchedPool, PoolConfig, GreenTaskBuilder};
    use std::io::signal::{Listener, Interrupt};
    use std::task::TaskBuilder;

    let mut config = PoolConfig::new();
    config.event_loop_factory = rustuv::event_loop;

    let mut pool = SchedPool::new(config);
    TaskBuilder::new().green(&mut pool).spawn(proc() {
        let mut l = Listener::new();
        l.register(Interrupt).unwrap();
        l.rx.recv();
    });
    pool.shutdown();
}

fn root(_req: &mut Request, _params: &Params) -> IoResult<Response> {
    let bytes = "you found the root!\n".as_bytes().to_owned();
    Ok(response(200i, HashMap::new(), MemReader::new(bytes)))
}

fn id(_req: &mut Request, params: &Params) -> IoResult<Response> {
    let string = format!("you found the id {}!\n", params["id"]);
    let bytes = string.into_bytes();

    Ok(response(200i, HashMap::new(), MemReader::new(bytes)))
}
