use std::{
    io::prelude::*,
    net::{TcpListener, TcpStream}, collections::HashMap,
};
use cw_grid_server::HttpRequest;
use tera::Tera;


fn main() {

    let tera = Tera::new("templates/**/*").unwrap();

    let mut routes: HashMap<String, fn(&HttpRequest, &Tera) -> String> = HashMap::new();

    routes.insert("/".to_string(), index_handler);
    routes.insert("/hello".to_string(), hello_handler);

    let api: Api = Api::register_routes( &tera, routes);

    let listener = TcpListener::bind("127.0.0.1:5051").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(stream, &api);
    }
}

fn handle_connection(mut stream: TcpStream, api: &Api) {

    let res = HttpRequest::new(&stream);
    println!("handling connection");
    if res.is_ok(){
        let req = res.unwrap();
        let response = api.handle_request(&req);
        stream.write_all(response.as_bytes()).unwrap()
    }
    else {
        stream.write_all("Not found".as_bytes()).unwrap()
    }
}

struct Api<'a> {
    routes: HashMap<String, fn(&HttpRequest, &'a Tera) -> String>,
    tera: &'a Tera
}

impl<'a> Api<'a>{

    fn handle_request(&self, req: &HttpRequest) -> String{
        println!("{:?}",req);
        match req {
            HttpRequest::Get { status_line, headers: _ } => {
                let handler = self.routes.get(&status_line.route).unwrap();
                handler(req, self.tera)
            },
            HttpRequest::Post { status_line, headers: _, body: _ } => {
                let handler = self.routes.get(&status_line.route).unwrap();
                handler(req, self.tera)
            },
        }

    }

    fn register_routes(tera: &'a Tera, routes:  HashMap<String, fn(&HttpRequest, &'a Tera) -> String>) -> Self {
        Self{routes, tera}
    }

}

fn hello_handler<'a>(_req: &HttpRequest, tera: &'a Tera) -> String{
    println!("hello route");
    let status_line = "HTTP/1.1 200 Internal Server Error";
    let mut context = tera::Context::new();
    context.insert("data", "Hello");
    let contents = tera.render("hello.html", &context).unwrap();
    let length = contents.len();
    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
    return response
}

fn index_handler<'a>(_req: &HttpRequest, tera: &'a Tera) -> String{
    println!("hello route");
    let status_line = "HTTP/1.1 200 Internal Server Error";
    let mut context = tera::Context::new();
    context.insert("data", "Index");
    let contents = tera.render("hello.html", &context).unwrap();
    let length = contents.len();
    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
    return response
}