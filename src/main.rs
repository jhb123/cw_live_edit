use std::{
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream}, num::NonZeroUsize, collections::HashMap,
};
use chrono::{DateTime, Utc};
use cw_grid_server::HttpRequest;
use tera::Tera;


struct Coordinate {
    x: usize,
    y: usize
}

impl Coordinate {
    pub fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }
}

struct GridData {
    letter : char,
    timestamp : DateTime<Utc>,
    coordinate : Coordinate
    
}


impl GridData {

    pub fn update(&mut self, new_letter: char, new_timestamp: DateTime<Utc>) {
        
        if new_letter != self.letter{
            if new_timestamp > self.timestamp {
                self.letter = new_letter;
                self.timestamp = new_timestamp;
            }
        }
    }
}

impl Default for GridData {
    fn default() -> Self {
        Self { 
            letter: '\u{1F601}',
            timestamp: Utc::now(),
            coordinate : Coordinate::new(0,0) 
        }
    }
}

struct Clue {
    name: String,
    cells: Vec::<GridData>
}

impl Clue {
    pub fn new(name: &str, cells: Vec::<GridData>) -> Self {
        Self { name: name.to_string(), cells}
    }
}

fn main() {

    let mut tera = Tera::new("templates/**/*").unwrap();

    let mut routes: HashMap<String, fn(&HttpRequest) -> String> = HashMap::new();

    routes.insert("/".to_string(), index_handler);
    routes.insert("/hello".to_string(), hello_handler);

    let api = Api::register_routes(routes);

    let listener = TcpListener::bind("127.0.0.1:5051").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(stream, &tera, &api);
    }
}

fn handle_connection(mut stream: TcpStream, tera: &Tera, api: &Api) {

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

fn route_matcher(route: &str, tera: &Tera, stream: &mut TcpStream){

    match route {
        "/hello" => {
            hello_response(tera, stream)
        },
        _ => {
            let status_line = "HTTP/1.1 404 Internal Server Error";
            let mut context = tera::Context::new();
            context.insert("status", "404");
            let contents = tera.render("error.html", &context).unwrap();
            let length = contents.len();
            let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
            stream.write_all(response.as_bytes()).unwrap();
        }
    }



}

fn hello_response (tera: &Tera, stream: &mut TcpStream) {
    let template_name = "hello.html";
    let mut context = tera::Context::new();
    context.insert("data", "Hello from template");

    match tera.render(&template_name, &context){
        Ok(contents) => {
            let status_line = "HTTP/1.1 200 OK";
            let length = contents.len();
            let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
            stream.write_all(response.as_bytes()).unwrap();
        },
        Err(data) => {
            let status_line = "HTTP/1.1 500 Internal Server Error";
            let mut context = tera::Context::new();
            context.insert("status", "500");
            let contents = tera.render("error.html", &context).unwrap();
            let length = contents.len();
            let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
            stream.write_all(response.as_bytes()).unwrap();
        },
    }


}

struct Api {
    routes: HashMap<String, fn(&HttpRequest) -> String>
}

impl Api {

    fn handle_request(&self, req: &HttpRequest) -> String{
        println!("{:?}",req);
        match req {
            HttpRequest::Get { status_line, headers: _ } => {
                let handler = self.routes.get(&status_line.route).unwrap();
                handler(req)
            },
            HttpRequest::Post { status_line, headers, body } => {
                let handler = self.routes.get(&status_line.route).unwrap();
                handler(req)
            },
        }

    }

    fn register_routes(routes:  HashMap<String, fn(&HttpRequest) -> String>) -> Self {
        Self{routes}
    }

}

fn hello_handler(req: &HttpRequest) -> String{
    println!("hello route");
    return "Hello".to_string()
}

fn index_handler(req: &HttpRequest) -> String{
    println!("index route");
    return "Index".to_string()
}