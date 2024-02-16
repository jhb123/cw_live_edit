use std::{
    collections::HashMap, default, fs::File, io::{prelude::*, BufReader}, net::{TcpListener, TcpStream}, sync::{mpsc::{self, Sender}, Arc, Mutex}, thread::{self, sleep, JoinHandle}, time::{self, Duration}
};
use cw_grid_server::{decode_client_frame, web_socket_accept, websocket_message, HttpRequest, HttpVerb, StatusLine, ThreadPool};
use regex::Regex;
use tera::Tera;
use log::{info, warn};
use lazy_static::lazy_static;

lazy_static! {
    static ref PUZZLE: Mutex<PuzzleChannel> = Mutex::new(PuzzleChannel::new());
}

type RouteMapping = HashMap<&'static str, fn(&HttpRequest,  Arc<Tera>, TcpStream)>;

fn main() {
    env_logger::init();

    info!("{:?}",*PUZZLE);

    //let foo = Regex::new(r"\d+").unwrap();

    let mut routes: RouteMapping = HashMap::new();
    routes.insert(r"^/$", index_handler);
    routes.insert(r"^/hello$", hello_handler);
    routes.insert(r"^/foo/\d+$", variable_request_test);

    routes.insert(r"^/crossword.js$", crossword_js);
    routes.insert(r"^/echo$", echo);
    routes.insert(r"^/puzzle/\d+$", echo);


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
    routes: RouteMapping,
    tera:  Arc<Tera>
}

impl Api{

    fn handle_request(&self, req: &HttpRequest, stream: TcpStream){

        let incoming_route = match req {
            HttpRequest::Get { status_line, headers:_ } => status_line.route.as_str(),
            HttpRequest::Post { status_line, headers: _, body: _ } => status_line.route.as_str(),
        };

        // base case
        
        info!("{req}");
        match req {
            HttpRequest::Get { status_line, headers: headers } => {
                // Regex::new()
                for (api_route, handler) in self.routes.iter() {
                    let reg = Regex::new(api_route).unwrap();
                    info!("regex:{}",reg);
                    if reg.is_match(incoming_route) { 
                        info!("Routing {incoming_route} to {api_route}");
                        return handler(req, Arc::clone(&self.tera), stream);
                    };
                }
                warn!("Didn't match any routes");
                missing( Arc::clone(&self.tera),stream)

            },
            HttpRequest::Post { status_line, headers: headers , body: body } => {
                for (route, handler) in self.routes.iter() {
                    let reg = Regex::new(route).unwrap();
                    if reg.is_match(route) { 
                        return handler(req, Arc::clone(&self.tera), stream);
                    };
                }
                missing( Arc::clone(&self.tera),stream)
            },
        }


    }

    fn register_routes(routes:  RouteMapping, tera:  Arc<Tera>) -> Self {
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

fn variable_request_test(req: &HttpRequest, tera: Arc<Tera>, mut stream: TcpStream){
    let response_status_line = "HTTP/1.1 200 Ok";
    info!("Response Status {}",response_status_line);
    let mut context = tera::Context::new();
    let status_line = match req {
        HttpRequest::Get { status_line, .. } => status_line,
        HttpRequest::Post { status_line,.. } => status_line,
    };

    let path_info = Regex::new(r"(?<num>\d+)").unwrap();
    let caps = path_info.captures(&status_line.route).unwrap();
    info!("{:?}",&caps["num"]);
    context.insert("data", &caps["num"]);
    let contents = tera.render("foo.html", &context).unwrap();
    let length = contents.len();
    let response = format!("{response_status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
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
    stream.set_read_timeout(Some(Duration::from_millis(10)));

    let stream = Arc::new(Mutex::new(stream));
    let (sender, receiver) = mpsc::channel::<Vec<u8>>();

    PUZZLE.lock().unwrap().add_new_client(sender);

    let stream_clone = Arc::clone(&stream);
    thread::spawn(move || {
        loop{
            let msg = receiver.recv().unwrap();
            unsafe {
                // who cares, this is just debugging        
                let frame = websocket_message(&String::from_utf8_unchecked(msg));
                stream_clone.lock().unwrap().write_all(&frame).unwrap();
            }
        }
    });

    let sender = PUZZLE.lock().unwrap().sender.clone();
    loop{
        {
            let mut st = stream.lock().unwrap();
            let mut buf_reader = BufReader::new(&mut *st);
            let _ = match decode_client_frame(&mut buf_reader) {
                Ok(msg) => sender.send(msg),
                Err(err) => {
                    Ok(())
                },
            };                
        }
        sleep(Duration::from_millis(10))
    }

}

// type PuzzlePool = HashMap<&str, PuzzleChannel>;
struct PuzzlePool {
    pool: HashMap<String, PuzzleChannel>
}

impl PuzzlePool{

    fn new() -> Self {
        let pool = HashMap::new();
        Self { pool }
    }

    fn connect(req: &HttpRequest, mut stream: TcpStream) {

        let handshake = Self::handshake(req).unwrap();
        stream.write_all(handshake.as_bytes()).unwrap();

        let route = match req {
            HttpRequest::Get { status_line, headers:_ } => status_line.route.as_str(),
            HttpRequest::Post { status_line, headers: _, body: _ } => status_line.route.as_str(),
        };

        // let;

    }

    fn handshake(req: &HttpRequest) -> Result<String, &str>{
        let headers = match req {
            HttpRequest::Get { status_line: _, headers } => headers,
            HttpRequest::Post { status_line: _, headers: _, body: _ } => return Err("Do not use post for this"),
        };
        
        let status_line = "HTTP/1.1 101 Switching Protocols";
        info!("Response Status {}",status_line);
        
        let sender_key = headers.get("Sec-WebSocket-Key").unwrap();
        let encoded_data = web_socket_accept(sender_key);
    
        let handshake = format!("{status_line}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {encoded_data}\r\n\r\n");
        log::info!("Handshake:\n{}", handshake);

        return Ok(handshake)
    }



}




#[derive(Debug)]
struct PuzzleChannel{
    sender: Arc<mpsc::Sender<Vec<u8>>>,
    clients: Arc<Mutex<Vec<mpsc::Sender<Vec<u8>>>>>,
    running: bool,
    join_handle : Option<JoinHandle<()>>
}

impl PuzzleChannel {

    fn new() -> Self {
        
        let (sender, receiver) = mpsc::channel::<Vec<u8>>();

        let running = true;
        let clients = Arc::new(Mutex::new(vec![]));
        let clients_clone = clients.clone();

        let join_handle = thread::spawn(move || {
            while running {
                let msg = Arc::new(receiver.recv().unwrap());
                unsafe {
                    let msg_clone = msg.clone();
                    // who cares, this is just debugging
                    info!("{}", String::from_utf8_unchecked(msg_clone.to_vec()));
                }
                let msg_clone = msg.clone();

                clients_clone.lock().unwrap()
                    .iter()
                    .for_each(|x: &Sender<Vec<u8>>| x.send(msg_clone.to_vec()).unwrap());
            }
            info!("finishing")
        });

        Self { sender: Arc::new(sender), clients, running, join_handle: Some(join_handle) }

    }

    fn add_new_client(&mut self, Sender: Sender<Vec<u8>>) {
        self.clients.lock().unwrap().push(Sender)
    }
}

impl Drop for PuzzleChannel {
    fn drop(&mut self) {
        self.running = false;
        self.join_handle.take().unwrap().join();
        info!("finished")
    }
}


