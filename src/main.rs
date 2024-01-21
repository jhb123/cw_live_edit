use std::{
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream}, num::NonZeroUsize,
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


    let listener = TcpListener::bind("127.0.0.1:5051").unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(stream, &tera);
    }
}

fn handle_connection(mut stream: TcpStream, tera: &Tera) {

    let res = HttpRequest::new(stream);

    if res.is_ok(){
        let req = res.unwrap();
        match req {
            HttpRequest::GET{..} => println!("{:?}",req),
            HttpRequest::POST{..} => println!("{:?}",req)
    
        }
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