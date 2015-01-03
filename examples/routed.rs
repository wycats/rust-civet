extern crate conduit;
extern crate civet;
extern crate "route-recognizer" as routing;

use std::collections::HashMap;
use std::fmt::Show;
use std::io::{IoResult, MemReader};
use std::sync::mpsc::channel;

use civet::{Config, Server, response};
use conduit::{Request, Response};
use routing::{Router, Params};

struct MyServer {
    router: Router<fn(&mut Request, &Params) -> IoResult<Response>>,
}

impl conduit::Handler for MyServer {
    fn call(&self, req: &mut Request) -> Result<Response, Box<Show + 'static>> {
        let hit = match self.router.recognize(req.path()) {
            Ok(m) => m,
            Err(e) => panic!("{}", e),
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
    let (_tx, rx) = channel::<()>();
    rx.recv();
}

fn root(_req: &mut Request, _params: &Params) -> IoResult<Response> {
    let bytes = b"you found the root!\n".to_vec();
    Ok(response(200i, HashMap::new(), MemReader::new(bytes)))
}

fn id(_req: &mut Request, params: &Params) -> IoResult<Response> {
    let string = format!("you found the id {}!\n", params["id"]);
    let bytes = string.into_bytes();

    Ok(response(200i, HashMap::new(), MemReader::new(bytes)))
}
