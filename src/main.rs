use std::{
    collections::HashMap, fs::File, io::{prelude::*, BufReader}, net::{TcpListener, TcpStream}, sync::Arc, thread, time::Duration
};
use cw_grid_server::{decode_client_frame, web_socket_accept, websocket_message, HttpRequest, ThreadPool};
use tera::Tera;
use log::info;

fn main() {
    env_logger::init();

    let mut routes: HashMap<&'static str, fn(&HttpRequest,  Arc<Tera>, TcpStream)> = HashMap::new();
    routes.insert("/", index_handler);
    routes.insert("/hello", hello_handler);
    routes.insert("/crossword.js", crossword_js);
    routes.insert("/echo", echo);
    
    let tera = Tera::new("templates/**/*").unwrap();
    let tera_arc = Arc::new(tera);
    
    let api: Api = Api::register_routes(routes, tera_arc);
    let api_arc = Arc::new(api);

    let pool = ThreadPool::new(4);

    let addr = "127.0.0.1:5051";

    let listener = TcpListener::bind(addr).unwrap();
    info!("Started on: http://{addr}");
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let api_arc_clone = Arc::clone(&api_arc);
        // let tera_arc_clone = Arc::clone(&tera_arc);
        pool.execute(|| {
            handle_connection(stream, api_arc_clone);
        });
        
    }
    
}

fn handle_connection(mut stream: TcpStream, api: Arc<Api>) {
    info!("handling connection");
    let res = HttpRequest::new(&stream);
    if res.is_ok(){
        let req = res.unwrap();
        api.handle_request(&req, stream);
    }
    else {
        stream.write_all("Not found".as_bytes()).unwrap()
    }
}

#[derive(Clone)]
struct Api {
    routes: HashMap<&'static str, fn(&HttpRequest,  Arc<Tera>, TcpStream)>,
    tera:  Arc<Tera>
}

impl Api{

    fn handle_request(&self, req: &HttpRequest, stream: TcpStream){
        info!("{req}");
        match req {
            HttpRequest::Get { status_line, headers: _ } => {
                match self.routes.get(&status_line.route as &str) {
                    Some(handler) => handler(req, Arc::clone(&self.tera), stream),
                    None => missing( Arc::clone(&self.tera), stream),
                }
                
            },
            HttpRequest::Post { status_line, headers: _, body: _ } => {
                match self.routes.get(&status_line.route as &str){
                    Some(handler) => handler(req, Arc::clone(&self.tera), stream),
                    None => missing( Arc::clone(&self.tera),stream),
                }
            },
        }

    }

    fn register_routes(routes:  HashMap<&'static str, fn(&HttpRequest,  Arc<Tera>, TcpStream)>, tera:  Arc<Tera>) -> Self {
        Self{routes, tera}
    }

}

fn hello_handler(_req: &HttpRequest, tera:  Arc<Tera>, mut stream: TcpStream){
    thread::sleep(Duration::from_secs(5));
    let status_line = "HTTP/1.1 200 Ok";
    info!("Response Status {}",status_line);
    let mut context = tera::Context::new();
    context.insert("data", "Hello");
    let contents = tera.render("hello.html", &context).unwrap();
    let length = contents.len();
    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
    stream.write_all(response.as_bytes()).unwrap();
}

fn index_handler(_req: &HttpRequest, tera: Arc<Tera>, mut stream: TcpStream){
    let status_line = "HTTP/1.1 200 Ok";
    info!("Response Status {}",status_line);
    let mut context = tera::Context::new();
    context.insert("data", "Index");
    let contents = tera.render("hello.html", &context).unwrap();
    let length = contents.len();
    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
    stream.write_all(response.as_bytes()).unwrap();
}

fn missing(tera: Arc<Tera>, mut stream: TcpStream){
    let status_line = "HTTP/1.1 404 Ok";
    info!("Response Status {}",status_line);
    let mut context = tera::Context::new();
    context.insert("status", "404");
    let contents = tera.render("error.html", &context).unwrap();
    let length = contents.len();
    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
    stream.write_all(response.as_bytes()).unwrap();
}

fn crossword_js(_req: &HttpRequest, _: Arc<Tera>, mut stream: TcpStream) {
    let status_line = "HTTP/1.1 200 Ok";
    info!("Response Status {}",status_line);
    let mut file = File::open("static/crossword.js").unwrap();
    let mut contents = String::new();
    let length = file.read_to_string(&mut contents).unwrap();
    let response = format!("{status_line}\r\nContent-Length: {length}\nContent-Type: text/javascript\r\n\r\n{contents}");
    stream.write_all(response.as_bytes()).unwrap();
}

fn echo(req: &HttpRequest, tera: Arc<Tera>, mut stream: TcpStream) {
    
    let headers = match req {
        HttpRequest::Get { status_line: _, headers } => headers,
        HttpRequest::Post { status_line: _, headers: _, body: _ } => return missing(tera,stream),
    };
    
    let status_line = "HTTP/1.1 101 Switching Protocols";
    info!("Response Status {}",status_line);
    
    let sender_key = headers.get("Sec-WebSocket-Key").unwrap();
    let encoded_data = web_socket_accept(sender_key);

    let handshake = format!("{status_line}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {encoded_data}\r\n\r\n");
    log::info!("Handshake:\n{}", handshake);

    stream.write_all(handshake.as_bytes()).unwrap();
    let frame = websocket_message("Hello from rust!");
    stream.write_all(&frame).unwrap();


    let mut buf_reader = BufReader::new(&mut stream);
    
    let msg = decode_client_frame(&mut buf_reader);
    unsafe {
        // who cares, this is just debugging
        info!("{}", String::from_utf8_unchecked(msg));
    }

}

