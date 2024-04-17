use cw_grid_server::{
    crossword::{Cell, Crossword}, db::{create_new_puzzle, create_puzzle_dir, get_all_puzzle_db, get_puzzle, get_puzzle_db, init_db, save_puzzle}, websockets::{close_websocket_message, decode_client_frame, websocket_handshake, websocket_message, OpCode}, HttpRequest, ThreadPool
};
use lazy_static::lazy_static;
use log::{error, info, trace, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap, env, fs::File, io::{prelude::*, BufReader, Error, ErrorKind}, net::{TcpListener, TcpStream}, sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
    }, thread::{self, sleep}, time::Duration
};
use tera::Tera;

lazy_static! {
    static ref PUZZLEPOOL: Mutex<PuzzlePool> = Mutex::new(PuzzlePool::new());
    static ref THREADPOOL: ThreadPool = ThreadPool::new(4);
}

struct HandlerError {
    stream: TcpStream,
    error: Error
}

impl HandlerError {
    fn new(stream: TcpStream, error: Error) -> Self {
        HandlerError {stream, error}
    }
}

type HandlerFn = fn(&HttpRequest, Arc<Tera>, TcpStream) -> Result<TcpStream, HandlerError>;
type RouteMapping = HashMap<&'static str, HandlerFn>;

fn main() {
    env_logger::init();

    if let Err(e) = create_puzzle_dir() {
        warn!("{}",e)
    }

    if let Err(e) = init_db(){
        warn!("{}",e)
    }
    
    info!("{:?}", *PUZZLEPOOL);

    let mut routes: RouteMapping = HashMap::new();
    routes.insert(r"^/$", index_handler);

    routes.insert(r"^/crossword.js$", crossword_js);
    routes.insert(r"^/crossword.html$", crossword_html);
    routes.insert(r"^/crossword.css$", crossword_css);
    routes.insert(r"^/styles.css$", styles_css);


    routes.insert(r"^/puzzle/\d+$", puzzle_handler);
    routes.insert(r"^/puzzle/\d+/data$", puzzle_handler_data);
    routes.insert(r"^/puzzle/\d+/live$", puzzle_handler_live);

    routes.insert(r"^/puzzle/add", puzzle_add_handler);


    let tera = Tera::new("templates/**/*").unwrap_or_else(|err| {
        error!("Sever failed to load templates: {}", err);
        std::process::exit(1);
    });

    let tera_arc = Arc::new(tera);

    let api: Api = Api::register_routes(routes, tera_arc);
    let api_arc = Arc::new(api);

    let port = env::var("PUZZLE_PORT").unwrap_or("5051".to_string());
    
    let addr = format!("0.0.0.0:{port}");

    let listener = TcpListener::bind(&addr).unwrap_or_else(|err| {
        error!("Sever failed to start on {addr}: {}", err);
        std::process::exit(1);
    });

    info!("Started on: http://{addr}");

    for stream in listener.incoming() {
        info!("Stream received");
        match stream {
            Ok(stream) => {
                let api_arc_clone = Arc::clone(&api_arc);
                match THREADPOOL.execute(|| {
                    handle_connection(stream, api_arc_clone);
                }) {
                    Ok(_) => info!("Succesfully handled connection"),
                    Err(e) => error!("Failed handled connection {0:?}", e),
                }
            },
            Err(e) => {
                error!("Connection to the stream failed: {e}")
            }
        }
        
    }
}

fn handle_connection(stream: TcpStream, api: Arc<Api>) {
    info!("handling connection");
    match HttpRequest::new(&stream) {
        Ok(req) => {
            api.handle_request(&req, stream);
        },
        Err(e) => {
            error!("Error handling request {e}");
            api.bad_request(stream).unwrap_or_else(|err| {
                match api.server_error(err.stream) {
                    Ok(s) => return s,
                    Err(e) => {
                        warn!("Could not send the internal server error page to the client: {}",e.error);
                        return e.stream
                    }
                }
            });
        }
    };
}

#[derive(Clone)]
struct Api {
    routes: RouteMapping,
    tera: Arc<Tera>,
}

impl Api {
    fn handle_request(&self, req: &HttpRequest, stream: TcpStream) {
        let incoming_route = match req {
            HttpRequest::Get {
                status_line,
                headers: _,
            } => status_line.route.as_str(),
            HttpRequest::Post {
                status_line,
                headers: _,
                body: _,
            } => status_line.route.as_str(),
        };

        info!("{req}");
        match req {
            HttpRequest::Get { .. } => {
                self.route_incoming_request(incoming_route, req, stream);
            }
            HttpRequest::Post { .. } => {
                self.route_incoming_request(incoming_route, req, stream);
            }
        }
    }

    fn route_incoming_request(&self, incoming_route: &str, req: &HttpRequest, stream: TcpStream) {
        trace!("Trying to match {} to a route in the API.", incoming_route);

        for (api_route, handler) in self.routes.iter() {
            let reg = if let Ok(reg) = Regex::new(api_route) { reg } else {
                error!("The api route defintion {:?} is not valid regex.", api_route);
                if let Err(e) = self.server_error(stream) {
                    warn!("Failed to send the client the server error page: {}", e.error);
                };
                return;
            };
            
            if reg.is_match(incoming_route) {
                info!("Routing {incoming_route} to {api_route}");
                
                if let Err(err) = handler(req, Arc::clone(&self.tera), stream) {
                    error!("The route handler threw an error {}", err.error);
                    if let Err(e) = self.server_error(err.stream) {
                        warn!("Failed to send the client the server error page: {}", e.error);
                    };
                    return;
                };
                return 
            };
        }
        trace!("{} Didn't match any routes", incoming_route);

        if let Err(err) = missing(Arc::clone(&self.tera), stream) {
            error!("No routes were found, but we missing route handler threw an error {}", err.error);
            if let Err(e) = self.server_error(err.stream) {
                warn!("Failed to send the client the server error page: {}", e.error);
            };
            return;
        }
    }
    
    fn register_routes(routes: RouteMapping, tera: Arc<Tera>) -> Self {
        Self { routes, tera }
    }

    fn bad_request(&self, mut stream: TcpStream) -> Result<TcpStream, HandlerError> {
        let status_line = "HTTP/1.1 400 Not Found";
        info!("Response Status {}", status_line);
        let mut context = tera::Context::new();
        context.insert("status", "400");
        let contents = match self.tera.render("error.html", &context) {
            Ok(contents) => contents,
            Err(error) => {
                error!("Could not render the bad request page: {}", error);
                return Err(HandlerError::new(stream, Error::new(ErrorKind::InvalidData, format!("Could not render the bad request page: {error}"))))
            },
        };
        let length = contents.len();
        let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
        match stream.write_all(response.as_bytes()) {
            Ok(_) => Ok(stream),
            Err(error) => Err(HandlerError::new(stream, error))
        }
    }

    fn server_error(&self, mut stream: TcpStream) -> Result<TcpStream, HandlerError> {
        let status_line = "HTTP/1.1 500 Internal Server Error";
        info!("Response Status {}", status_line);
        let mut context = tera::Context::new();
        context.insert("status", "500");
        let contents = self.tera.render("error.html", &context).unwrap_or_else(|err| {
            error!("Could not render error template: {0}", err);
            "500 - Internal Server Error".to_string()
        });
        let length = contents.len();
        let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
        match stream.write_all(response.as_bytes()) {
            Ok(_) => Ok(stream),
            Err(error) => Err(HandlerError::new(stream, error))
        }
    }

}

fn index_handler(_req: &HttpRequest, tera: Arc<Tera>, mut stream: TcpStream) -> Result<TcpStream, HandlerError> {
    let status_line = "HTTP/1.1 200 Ok";
    info!("Response Status {}", status_line);
    let mut context = tera::Context::new();

    let puzzle_data = get_all_puzzle_db().unwrap();
    
    context.insert("data", "Index");
    context.insert("puzzles", &puzzle_data);

    let contents = tera.render("hello.html", &context).unwrap();
    let length = contents.len();
    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
    match stream.write_all(response.as_bytes()) {
        Ok(_) => Ok(stream),
        Err(error) => Err(HandlerError::new(stream, error))
    }
}


fn missing(tera: Arc<Tera>, mut stream: TcpStream) -> Result<TcpStream, HandlerError> {
    let status_line = "HTTP/1.1 404 Not Found";
    info!("Response Status {}", status_line);
    let mut context = tera::Context::new();
    context.insert("status", "404");
    let contents = tera.render("error.html", &context).unwrap();
    let length = contents.len();
    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
    match stream.write_all(response.as_bytes()) {
        Ok(_) => Ok(stream),
        Err(error) => Err(HandlerError::new(stream, error))
    }
}

fn crossword_js(_req: &HttpRequest, _: Arc<Tera>, stream: TcpStream)  -> Result<TcpStream, HandlerError> {
    static_file_handler(
        stream,"static/crossword.js","text/javascript")
}
fn crossword_html(_req: &HttpRequest, _: Arc<Tera>, stream: TcpStream)  -> Result<TcpStream, HandlerError> {
    static_file_handler(stream, "static/crossword.html","text/html")
}
fn crossword_css(_req: &HttpRequest, _: Arc<Tera>, stream: TcpStream)  -> Result<TcpStream, HandlerError> {
    static_file_handler(stream, "static/crossword.css","text/css")
}

fn styles_css(_req: &HttpRequest, _: Arc<Tera>, stream: TcpStream)  -> Result<TcpStream, HandlerError> {
    static_file_handler(stream, "static/styles.css","text/css")
}

fn static_file_handler(mut stream: TcpStream, path: &str, content_type: &str) -> Result<TcpStream, HandlerError> {
    let status_line = "HTTP/1.1 200 Ok";
    info!("Response Status {}", status_line);
    let mut file = File::open(path).unwrap();
    let mut contents = String::new();
    let length = file.read_to_string(&mut contents).unwrap();
    let response = format!("{status_line}\r\nContent-Length: {length}\nContent-Type: {content_type}\r\n\r\n{contents}");
    match stream.write_all(response.as_bytes()) {
        Ok(_) => Ok(stream),
        Err(error) => Err(HandlerError::new(stream, error))
    }
}

fn puzzle_handler(req: &HttpRequest, tera: Arc<Tera>, mut stream: TcpStream) -> Result<TcpStream, HandlerError> {
    // acquire the html of the page.
    let status_line = match req {
        HttpRequest::Get { status_line, .. } => status_line,
        HttpRequest::Post { status_line, .. } => status_line,
    };

    let path_info = Regex::new(r"(?<num>\d+)").unwrap();
    let caps = path_info.captures(&status_line.route).unwrap();
    let puzzle_num = caps["num"].to_string();

    let response_status_line = "HTTP/1.1 200 Ok";
    info!("Response Status {}", response_status_line);
    let mut context = tera::Context::new();
    context.insert("src", &format!("/puzzle/{puzzle_num}"));
    let data = get_puzzle_db(&puzzle_num).unwrap();

    context.insert("name", &format!("{}",data.name));
    let contents = tera.render("crossword.html", &context).unwrap();
    let length = contents.len();
    let response = format!("{response_status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
    match stream.write_all(response.as_bytes()) {
        Ok(_) => Ok(stream),
        Err(error) => Err(HandlerError::new(stream, error))
    }
}

fn puzzle_handler_data(req: &HttpRequest, _tera: Arc<Tera>, stream: TcpStream) -> Result<TcpStream, HandlerError>  {

    let status_line = match req {
        HttpRequest::Get { status_line, .. } => status_line,
        HttpRequest::Post { status_line, .. } => status_line,
    };

    let path_info = Regex::new(r"(?<num>\d+)/data").unwrap();
    let caps = path_info.captures(&status_line.route).unwrap();
    let puzzle_num = caps["num"].to_string();

    PUZZLEPOOL.lock().unwrap().get_grid_data(puzzle_num , &stream);
    Ok(stream)
}

fn puzzle_handler_live(req: &HttpRequest, _tera: Arc<Tera>, mut stream: TcpStream) -> Result<TcpStream, HandlerError> {

    let status_line = match req {
        HttpRequest::Get { status_line, .. } => status_line,
        HttpRequest::Post { status_line, .. } => status_line,
    };

    let path_info = Regex::new(r"(?<num>\d+)/live").unwrap();
    let caps = path_info.captures(&status_line.route).unwrap();
    let puzzle_num = caps["num"].to_string();

    let handshake = websocket_handshake(req).unwrap();

    if let Err(error) = stream.write_all(handshake.as_bytes()) {
        return Err(HandlerError::new(stream, error))
    }

    // pass info into the puzzle pool so that this request can be routed to the correct puzzle channel
    match PUZZLEPOOL.lock().unwrap().connect_client(puzzle_num, stream) {
        Ok(stream) => {
            return Ok(stream)
        }
        Err(mut handler) => {
            let data = close_websocket_message();
            handler.stream.write_all(&data).unwrap();
            return Err(HandlerError::new(handler.stream, Error::from(ErrorKind::Other)))
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct AddPuzzleBody{
    name: String,
    crossword: Crossword
}

fn puzzle_add_handler(req: &HttpRequest, _tera: Arc<Tera>, mut stream: TcpStream) -> Result<TcpStream, HandlerError> {

    let _status_line = match req {
        HttpRequest::Get {  .. } => todo!("Throw a useful error"),
        HttpRequest::Post { status_line: _, headers: _, body} => {

            let body = String::from_utf8(body.clone()).unwrap();
            let request_data: AddPuzzleBody  = serde_json::from_str(&body).unwrap();


            let _ = create_new_puzzle(&request_data.name, &request_data.crossword);

            let response_status_line = "HTTP/1.1 200 Ok";
            info!("Response Status {}", response_status_line);
            let contents = serde_json::to_string(&request_data.crossword).unwrap();
            let length = contents.len();
            let response = format!("{response_status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
            match stream.write_all(response.as_bytes()) {
                Ok(_) => return Ok(stream),
                Err(error) => return Err(HandlerError::new(stream, error))
            }

        },
    };

}


#[derive(Debug)]
struct PuzzlePool {
    pool: HashMap<String, Arc<Mutex<PuzzleChannel>>>,
}

impl PuzzlePool {
    fn new() -> Self {
        let pool = HashMap::new();
        Self { pool }
    }

    fn connect_client(&mut self, puzzle_num: String, stream: TcpStream) -> Result<TcpStream, HandlerError> {


        match self.pool.get(&puzzle_num) {
            Some(puzzle_channel) => {
                info!("Connecting websocket client to existing puzzle.");
                route_stream_to_puzzle(puzzle_channel.clone(), stream)
            }
            None => {
                info!("No channel found to route websocket client. Creating a new channel");
                match PuzzleChannel::new(puzzle_num.clone()){
                    Ok(channel) => {
                        match channel {
                            Some(channel) => {
                                let new_channel = Arc::new(Mutex::new(channel));
                                self.pool.insert(puzzle_num, new_channel.clone());
                                route_stream_to_puzzle(new_channel.clone(), stream)
                            },
                            None => {
                                Err( HandlerError::new(stream, Error::from(ErrorKind::Other)))
                            }
                        }
                    },
                    
                    Err(_) => Err( HandlerError::new(stream, Error::from(ErrorKind::Other))),
                }
            }
        }
    }

    fn get_grid_data(&mut self, puzzle_num: String, mut stream: &TcpStream) {
        self.pool.iter().for_each(|(name,_)|{
            info!("channel {}",name)
        });
        match self.pool.get(&puzzle_num) {
            Some(puzzle_channel) => {
                // get crossword from channel
                info!("Puzzle channel found. Sending puzzle channel data.");
                puzzle_channel.lock().unwrap().send_puzzle(stream);
            }
            None => {
                info!("Puzzle channel not found. Loading data from disk");

                match get_puzzle(&puzzle_num) {
                    Ok(grid) => {
                        let status_line = "HTTP/1.1 200 Ok";
                        info!("Response Status {}", status_line);
                        // let grid = Crossword::demo_grid();
                        let contents = serde_json::to_string(&grid).unwrap();
                        let length = contents.len();
                        let response = format!("{status_line}\r\nContent-Length: {length}\r\nContent-Type: application/json\r\n\r\n{contents}");
                        stream.write_all(response.as_bytes());
                    },
                    Err(e) => {
                        let status_line = "HTTP/1.1 404 Not Found";
                        info!("Response Status {}", status_line);
                        // let grid = Crossword::demo_grid();
                        let contents = format!("{{\"error\" : {} }}",e);
                        let length = contents.len();
                        let response = format!("{status_line}\r\nContent-Length: {length}\r\nContent-Type: application/json\r\n\r\n{contents}");
                        stream.write_all(response.as_bytes());
                    }
                }
            }
        }
    }

    fn remove_channel(&mut self, puzzle_num: &str) {
        self.pool.remove(puzzle_num);
    }
}

type ThreadSafeSenderVector = Arc<Mutex<Vec<Arc<mpsc::Sender<Vec<u8>>>>>>;

#[derive(Debug)]
struct PuzzleChannel {
    channel_wide_sender: Arc<mpsc::Sender<Vec<u8>>>,
    clients: ThreadSafeSenderVector,
    terminate_sender: mpsc::Sender<bool>,
    crossword: Arc<Mutex<Crossword>>,
    puzzle_num: String
}

impl PuzzleChannel {
    fn new(puzzle_num: String) -> Result<Option<Self>, Error> {
        let puzzle_num_clone = puzzle_num.clone();

        let (sender, receiver) = mpsc::channel::<Vec<u8>>();

        let (terminate_sender, terminate_rec) = mpsc::channel();

        let clients: ThreadSafeSenderVector = Arc::new(Mutex::new(vec![]));
        let clients_clone = clients.clone();

        let crossword = match get_puzzle(&puzzle_num)? {
            Some(data) =>  Arc::new(Mutex::new(data)),
            None => {
                warn!("Cannot make a new puzzle channel as there is no crossword data");
                return Ok(None)
            } 
        };

        let crossword_clone = crossword.clone();

        match THREADPOOL.execute(move || {
            loop {
                if let Ok(should_break) = terminate_rec.recv_timeout(Duration::from_millis(10)){
                    if should_break {
                        info!("Puzzle channel received termination signal");
                        break
                    }
                    else {
                        info!("Puzzle channel received continuation signal");
                    }
                }

                let msg = Arc::new(receiver.recv().unwrap());
                unsafe {
                    let msg_clone = msg.clone();
                    // who cares, this is just debugging
                    info!("{}", String::from_utf8_unchecked(msg_clone.to_vec()));
                }
                
                match String::from_utf8(msg.to_vec()) {
                    Ok(client_data) => {
                        let incoming_data: Result<Cell, serde_json::Error>  = serde_json::from_str(&client_data);
                        match incoming_data {
                            Ok(deserialised) => {
                                crossword_clone.lock().unwrap().update_cell(deserialised);
                            },
                            Err(_) => {
                                warn!("cannot deserialise")
                            },
                        }
                        
                    },
                    Err(e) => info!("{}", e),
                }

                let msg_clone = msg.clone();

                clients_clone
                    .lock()
                    .unwrap()
                    .iter()
                    .filter_map(|x| x.send(msg_clone.to_vec()).err())
                    .for_each(drop);
            }
            info!("finishing");
            PUZZLEPOOL.lock().unwrap().remove_channel(&puzzle_num);

        })
        {
            Ok(_) => info!("Succesfully exceuted puzzle channel creation"),
            Err(e) => info!("Failed to exceuted puzzle channel creation {0:?}", e),
        }

        Ok(Some(Self {
            channel_wide_sender: Arc::new(sender),
            clients,
            terminate_sender,
            crossword,
            puzzle_num: puzzle_num_clone
        }))
    }

    fn add_new_client(&mut self, sender: Arc<Sender<Vec<u8>>>) {
        info!("adding new client to senders");
        self.clients.lock().unwrap().push(sender)
    }

    fn remove_client(&mut self, sender: &Arc<Sender<Vec<u8>>>) {
        let mut clients = self.clients.lock().unwrap();
        if let Some(idx) = clients.iter().position(|x| Arc::ptr_eq(x, sender)) {
            clients.remove(idx);
            info!("found client, removing")
        }

        info!("number of remaining clients: {}",clients.len());
        if clients.len() == 0 {
            info!("terminating channel");
            self.terminate_sender.send(true);
        }

    }


    fn send_puzzle(&self, mut stream: &TcpStream){
        let status_line = "HTTP/1.1 200 Ok";
        info!("Response Status {}", status_line);
        let grid = self.crossword.lock().unwrap();
        let contents = serde_json::to_string(&*grid).unwrap();
        let length = contents.len();
        let response = format!("{status_line}\r\nContent-Length: {length}\r\nContent-Type: application/json\r\n\r\n{contents}");
        stream.write_all(response.as_bytes());
    }

}

impl Drop for PuzzleChannel {
    fn drop(&mut self) {

        let data = self.crossword.lock().unwrap();
        save_puzzle(&self.puzzle_num ,&data);
        info!("dropping puzzle channel")
    }
}


fn route_stream_to_puzzle(puzzle_channel: Arc<Mutex<PuzzleChannel>>, stream: TcpStream) -> Result<TcpStream, HandlerError>{

    let stream_clone = stream.try_clone().unwrap();

    let _ = stream.set_read_timeout(Some(Duration::from_millis(10)));
    
    let stream_arc = Arc::new(Mutex::new(stream));
    let (sender, receiver) = mpsc::channel::<Vec<u8>>();

    let (terminate_sender, terminate_rec) = mpsc::channel();

    let sender = Arc::new(sender);
    let sender_clone = sender.clone();
    {
        puzzle_channel.lock().unwrap().add_new_client(sender);
    }

    let channel_wide_sender = puzzle_channel.lock().unwrap().channel_wide_sender.clone();
    // let mut set = HashMap::new();

    let stream_writer = Arc::clone(&stream_arc);
    let rec_thread = thread::spawn( move || {
        loop {
            if let Ok(should_break) = terminate_rec.recv_timeout(Duration::from_millis(10)){
                if should_break {
                    info!("ending here");
                    break
                }
                else {
                    info!("not breaking here");
                }
            }
            let msg = receiver.recv().unwrap();
            match String::from_utf8(msg){
                Ok(s) => {
                    let frame = websocket_message(&s);
                    stream_writer.lock().unwrap().write_all(&frame).unwrap();
                }
                Err(e) => warn!("cannot turn msg ({:?}) into utf-8 string", e),
            }
        }
        info!("finished writing data to client");
        puzzle_channel.lock().unwrap().remove_client(&sender_clone);

    });

    let send_thread = thread::spawn(move || {
        loop {
            {
                let mut st = stream_arc.lock().unwrap();
                let mut buf_reader = BufReader::new(&mut *st);
                let _ = match decode_client_frame(&mut buf_reader) {
                    Ok(msg) => {
                        // put here!
                        match msg.opcode {
                            OpCode::Continuation => todo!(),
                            OpCode::Ping => todo!(),
                            OpCode::Pong => todo!(),
                            OpCode::Close => {
                                channel_wide_sender.send(msg.body).unwrap();
                                let frame = close_websocket_message();
                                st.write_all(&frame).unwrap();
                                terminate_sender.send(true).unwrap();
                                break
                            },
                            OpCode::Reserved => panic!("Cannot handle this opcode"),
                            OpCode::Text => channel_wide_sender.send(msg.body),
                            OpCode::Binary => channel_wide_sender.send(msg.body),
                        }
                    },
                    Err(_err) => {
                        Ok(())
                        // panic!("{}", err)
                    },
                };
            }
            sleep(Duration::from_millis(10))
        }
        // puzzle_channel;
        info!("finished reading websocket from client");

    });


    info!("Routed client to puzzle");
    Ok(stream_clone)
}