use cw_grid_server::{
    crossword::{Cell, Crossword}, db::{add_user, create_new_puzzle, create_puzzle_dir, get_all_puzzle_db, get_puzzle, get_puzzle_db, get_user_password, init_db, save_puzzle, set_session, validate_password}, get_form_data, get_login_cookies, is_authorised, response::{internal_error_response, ResponseBuilder, StatusCode}, websockets::{close_websocket_message, decode_client_frame, websocket_handshake, Message, OpCode}, HttpRequest, ThreadPool
};
use lazy_static::lazy_static;
use log::{error, info, trace, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap, env, fs::File, io::{prelude::*, BufReader, Error, ErrorKind}, net::{TcpListener, TcpStream}, sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
    }, thread::sleep, time::Duration
};
use tera::Tera;

lazy_static! {
    static ref PUZZLEPOOL: Mutex<PuzzlePool> = Mutex::new(PuzzlePool::new());
    static ref THREADPOOL: ThreadPool = {
        let num_threads = env::var("PUZZLE_THREADS")
            .unwrap_or_else(|_| "32".to_string())
            .parse::<usize>()
            .expect("CW_THREADS must be a valid number");
            ThreadPool::new(num_threads)
    };
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

    let mut routes: RouteMapping = HashMap::new();
    routes.insert(r"^/$", index_handler);

    routes.insert(r"^/crossword.js$", crossword_js);
    routes.insert(r"^/dialog.js$", dialog_js);
    routes.insert(r"^/crossword.html$", crossword_html);
    routes.insert(r"^/crossword.css$", crossword_css);
    routes.insert(r"^/styles.css$", styles_css);


    routes.insert(r"^/puzzle/\d+$", puzzle_handler);
    routes.insert(r"^/puzzle/\d+/data$", puzzle_handler_data);
    routes.insert(r"^/puzzle/\d+/live$", puzzle_handler_live);

    routes.insert(r"^/puzzle/add", puzzle_add_handler);
    routes.insert(r"^/puzzle/list$", puzzle_list_handler);

    // routes.insert(r"^/login", login_handler);
    routes.insert(r"^/sign-up", sign_up_handler);
    routes.insert(r"^/log-in", log_in_handler);
    routes.insert(r"^/log-out", log_out_handler);

    routes.insert(r"^/client-test", client_test_handler);
    routes.insert(r"^/add-client-test", add_client_test_handler);



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

    println!("Started on: http://{addr}");

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
            trace!("Finished handling request with api");
        },
        Err(e) => {
            error!("Error handling request {e}");
            api.bad_request(stream, &format!("Error handling request {e}")).unwrap_or_else(|err| {
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

        if let Err(err) = not_found(Arc::clone(&self.tera), stream, None) {
            error!("No routes were found, but the missing route handler threw an error: {}", err.error);
            if let Err(e) = self.server_error(err.stream) {
                warn!("Failed to send the client the server error page: {}", e.error);
            };
            return;
        }
    }
    
    fn register_routes(routes: RouteMapping, tera: Arc<Tera>) -> Self {
        Self { routes, tera }
    }

    fn bad_request(&self, stream: TcpStream, message: &str) -> Result<TcpStream, HandlerError> {
        let r = bad_request(Arc::clone(&self.tera), stream, message);
        trace!("Handled bad request");
        return r
    }

    fn server_error(&self, stream: TcpStream) -> Result<TcpStream, HandlerError> {
        return server_error(Arc::clone(&self.tera),stream)
    }

}

fn server_error(tera: Arc<Tera>, mut stream: TcpStream) -> Result<TcpStream, HandlerError> {
    let mut context = tera::Context::new();
    context.insert("status", "500");
    context.insert("message", "Internal Server Error");
    let contents = tera.render("error.html", &context).unwrap_or_else(|err| {
        error!("Could not render error template: {0}", err);
        "500 - Internal Server Error".to_string()
    });

    let response = internal_error_response(&contents);

    match stream.write_all(response.as_bytes()) {
        Ok(_) => Ok(stream),
        Err(error) => Err(HandlerError::new(stream, error))
    }
}

fn index_handler(req: &HttpRequest, tera: Arc<Tera>, mut stream: TcpStream) -> Result<TcpStream, HandlerError> {

    match req {
        HttpRequest::Get { status_line: _, headers } => {
            
            let mut context = tera::Context::new();
            let puzzle_data = match get_all_puzzle_db(){
                Ok(puzzle_data) => puzzle_data,
                Err(error) => return Err(HandlerError::new(stream, Error::new(ErrorKind::Other, format!("{}",error))))
            };

            match is_authorised(headers) {
                Ok(_) => {
                    context.insert("logged_in", &true);
                    context.insert("data", "Logged In");
                },
                Err(e) => {
                    context.insert("data", &e)
                },
            };

            
            context.insert("puzzles", &puzzle_data);
            let contents = match tera.render("index.html", &context){
                Ok(contents) => contents,
                Err(error) => return Err(HandlerError::new(stream, Error::new(ErrorKind::Other, format!("{}",error))))
            };

            let response = ResponseBuilder::new()
                .set_status_code(StatusCode::Ok)
                .set_html_content(contents)
                .build();
            
            match stream.write_all(response.as_bytes()) {
                Ok(_) => Ok(stream),
                Err(error) => Err(HandlerError::new(stream, error))
            }
        },
        HttpRequest::Post { status_line: _, headers: _, body: _ } => {
            return bad_request(tera, stream, "method not supported")
        }
    }    
}

fn not_found(tera: Arc<Tera>, mut stream: TcpStream, message: Option<&str>) -> Result<TcpStream, HandlerError> {
    let mut context = tera::Context::new();
    context.insert("status", "404");
    context.insert("message", message.unwrap_or("Not Found"));
    let contents = match tera.render("error.html", &context){
        Ok(contents) => contents,
        Err(error) => return Err(HandlerError::new(stream, Error::new(ErrorKind::Other, format!("{}",error))))
    };

    let response = ResponseBuilder::new()
        .set_status_code(StatusCode::NotFound)
        .set_html_content(contents)
        .build();

    match stream.write_all(response.as_bytes()) {
        Ok(_) => Ok(stream),
        Err(error) => Err(HandlerError::new(stream, error))
    }
}

fn bad_request(tera: Arc<Tera>, mut stream: TcpStream, message: &str) -> Result<TcpStream, HandlerError> {
    let mut context = tera::Context::new();
    context.insert("status", "400");
    context.insert("message", message );
    let contents = match tera.render("error.html", &context){
        Ok(contents) => contents,
        Err(error) => return Err(HandlerError::new(stream, Error::new(ErrorKind::Other, format!("{}",error))))
    };

    let response = ResponseBuilder::new()
        .set_status_code(StatusCode::BadRequest)
        .set_html_content(contents)
        .build();

    match stream.write_all(response.as_bytes()) {
        Ok(_) => Ok(stream),
        Err(error) => Err(HandlerError::new(stream, error))
    }
}

fn crossword_js(_req: &HttpRequest, _: Arc<Tera>, stream: TcpStream)  -> Result<TcpStream, HandlerError> {
    static_file_handler(stream, "static/crossword.js","text/javascript")
}

fn dialog_js(_req: &HttpRequest, _: Arc<Tera>, stream: TcpStream)  -> Result<TcpStream, HandlerError> {
    static_file_handler(stream, "static/dialog.js","text/javascript")
}

fn crossword_html(_req: &HttpRequest, _: Arc<Tera>, stream: TcpStream)  -> Result<TcpStream, HandlerError> {
    static_file_handler(stream, "static/crossword.html","text/html")
}
fn crossword_css(_req: &HttpRequest, _: Arc<Tera>, stream: TcpStream)  -> Result<TcpStream, HandlerError> {
    static_file_handler(stream, "static/crossword.css","text/css")
}

fn styles_css(_req: &HttpRequest, _: Arc<Tera>, stream: TcpStream)  -> Result<TcpStream, HandlerError> {
    static_file_handler(stream,"static/styles.css","text/css")
}

fn static_file_handler(mut stream: TcpStream, path: &str, content_type: &str) -> Result<TcpStream, HandlerError> {
    let mut file = match File::open(path){
        Ok(file) => file,
        Err(error) => return Err(HandlerError::new(stream, error))
    };

    let mut contents = String::new();
    if let Err(error) = file.read_to_string(&mut contents) {
        return Err(HandlerError::new(stream, error))
    };
    
    let response = ResponseBuilder::new()
        .set_status_code(StatusCode::Ok)
        .set_content(contents, content_type)
        .build();

    match stream.write_all(response.as_bytes()) {
        Ok(_) => Ok(stream),
        Err(error) => Err(HandlerError::new(stream, error))
    }
}

fn sign_up_handler(req: &HttpRequest, tera: Arc<Tera>, mut stream: TcpStream) -> Result<TcpStream, HandlerError> {

    match req {
        HttpRequest::Get { status_line: _, headers: _ } => {
            let context = tera::Context::new();
            let contents = match tera.render("signup.html", &context){
                Ok(contents) => contents,
                Err(error) => return Err(HandlerError::new(stream, Error::new(ErrorKind::Other, format!("{}",error))))
            };
        
            let response = ResponseBuilder::new()
                .set_status_code(StatusCode::NotFound)
                .set_html_content(contents)
                .build();
        
            match stream.write_all(response.as_bytes()) {
                Ok(_) => Ok(stream),
                Err(error) => Err(HandlerError::new(stream, error))
            }
        },
        HttpRequest::Post { status_line: _, headers: _, body } => {
            let body = match std::str::from_utf8(&body) {
                Ok(s) => s,
                Err(_) => {
                    return bad_request(tera, stream, "Body of the request was not valid UTF-8")
                },
            };

            let form_data = match get_form_data(body) {
                Ok(s) => s,
                Err(_) => return server_error(tera, stream)
            };

            let username = match form_data.get("username") {
                Some(x) => match *x {
                    Some(x) => x,
                    None => return bad_request(tera, stream, "Empty username field"),
                },
                None => return bad_request(tera, stream, "Missing username field")
            };
            let password = match form_data.get("password") {
                Some(x) => match *x {
                    Some(x) => x,
                    None => return bad_request(tera, stream, "Empty password field"),
                },
                None => return bad_request(tera, stream, "Missing password field")
            };
            let repeat_password = match form_data.get("repeatPassword") {
                Some(x) => match *x {
                    Some(x) => x,
                    None => return bad_request(tera, stream, "Empty repeat password field"),
                },
                None => return bad_request(tera, stream, "Missing repeat password field")
            };

            if password != repeat_password {
                return bad_request(tera, stream, "Passwords did not match")
            }


            let user_id = match add_user(username, password) {
                Ok(x) => x,
                Err(error) => {
                    match error {
                        rusqlite::Error::SqliteFailure(_, _) =>  return bad_request(tera, stream, "Username is not unique"),
                        _ => return Err(HandlerError::new(stream, Error::new(ErrorKind::InvalidData, error))),
                    }
                }
            };

            let session = match set_session(user_id) {
                Ok(x) => x,
                Err(error) => return Err(HandlerError::new(stream, Error::new(ErrorKind::Other, format!("{}",error))))
            };

            let mut context = tera::Context::new();
            let puzzle_data = match get_all_puzzle_db(){
                Ok(puzzle_data) => puzzle_data,
                Err(error) => return Err(HandlerError::new(stream, Error::new(ErrorKind::Other, format!("{}",error))))
            };
            context.insert("logged_in", &true);
            context.insert("data", &format!("Welcome back {}",username));
            context.insert("puzzles", &puzzle_data);
            let contents = match tera.render("index_content.html", &context){
                Ok(contents) => contents,
                Err(error) => return Err(HandlerError::new(stream, Error::new(ErrorKind::Other, format!("{}",error))))
            };

            let (session_cookie, username_cookie) = get_login_cookies(session, user_id);

            let response = ResponseBuilder::new()
                .set_status_code(StatusCode::Accepted)
                .set_html_content(contents)
                .add_cookie(session_cookie)
                .add_cookie(username_cookie)
                .build();
            
            match stream.write_all(response.as_bytes()) {
                Ok(_) => Ok(stream),
                Err(error) => Err(HandlerError::new(stream, error))
            }

        },
    }    
}

fn log_out_handler(_req: &HttpRequest, tera: Arc<Tera>, mut stream: TcpStream) -> Result<TcpStream, HandlerError> {

    let mut context = tera::Context::new();
    let puzzle_data = match get_all_puzzle_db(){
        Ok(puzzle_data) => puzzle_data,
        Err(error) => return Err(HandlerError::new(stream, Error::new(ErrorKind::Other, format!("{}",error))))
    };
    context.insert("logged_in", &false);
    context.insert("puzzles", &puzzle_data);
    let contents = match tera.render("index_content.html", &context){
        Ok(contents) => contents,
        Err(error) => return Err(HandlerError::new(stream, Error::new(ErrorKind::Other, format!("{}",error))))
    };

    let (session_cookie, username_cookie) = get_login_cookies(-1, -1);

    let response = ResponseBuilder::new()
                .set_status_code(StatusCode::Accepted)
                .set_html_content(contents)
                .add_cookie(session_cookie)
                .add_cookie(username_cookie)
                .build();

    match stream.write_all(response.as_bytes()) {
        Ok(_) => Ok(stream),
        Err(error) => Err(HandlerError::new(stream, error))
    }
}

fn log_in_handler(req: &HttpRequest, tera: Arc<Tera>, mut stream: TcpStream) -> Result<TcpStream, HandlerError> {

    match req {
        HttpRequest::Get { status_line: _, headers:_  } => {
            let context = tera::Context::new();
            let contents = match tera.render("login.html", &context){
                Ok(contents) => contents,
                Err(error) => return Err(HandlerError::new(stream, Error::new(ErrorKind::Other, format!("{}",error))))
            };
        
            let response = ResponseBuilder::new()
                .set_status_code(StatusCode::NotFound)
                .set_html_content(contents)
                .build();
        
            match stream.write_all(response.as_bytes()) {
                Ok(_) => Ok(stream),
                Err(error) => Err(HandlerError::new(stream, error))
            }
        },
        HttpRequest::Post { status_line: _, headers: _, body } => {
            let body = match std::str::from_utf8(&body) {
                Ok(s) => s,
                Err(_) => {
                    return bad_request(tera, stream, "Body of the request was not valid UTF-8")
                },
            };
            let form_data = match get_form_data(body) {
                Ok(s) => s,
                Err(e) => return bad_request(tera, stream, &e.to_string())
            };

            let username = match form_data.get("username") {
                Some(x) => match *x {
                    Some(x) => x,
                    None => return bad_request(tera, stream, "Empty username field"),
                },
                None => return bad_request(tera, stream, "Missing username field")
            };
            let password = match form_data.get("password") {
                Some(x) => match *x {
                    Some(x) => x,
                    None => return bad_request(tera, stream, "Empty password field"),
                },
                None => return bad_request(tera, stream, "Missing password field")
            };

            let sign_in = match get_user_password(username) {
                Ok(s) => {
                    info!("Successfully got password");
                    s
                },
                Err(e) => {
                    info!("{:?}",e);
                    return bad_request(tera, stream, &format!("{} Incorrect password",username))
                }
            };

            if let Err(_) = validate_password(password, &sign_in.password) {
                return bad_request(tera, stream, &format!("Wrong password"))
            }

            let session = match set_session(sign_in.id) {
                Ok(x) => x,
                Err(error) => return Err(HandlerError::new(stream, Error::new(ErrorKind::Other, format!("{}",error))))
            };


            let mut context = tera::Context::new();
            let puzzle_data = match get_all_puzzle_db(){
                Ok(puzzle_data) => puzzle_data,
                Err(error) => return Err(HandlerError::new(stream, Error::new(ErrorKind::Other, format!("{}",error))))
            };
            context.insert("data", &format!("Welcome back {}",username));
            context.insert("logged_in", &true);
            context.insert("puzzles", &puzzle_data);
            let contents = match tera.render("index_content.html", &context){
                Ok(contents) => contents,
                Err(error) => return Err(HandlerError::new(stream, Error::new(ErrorKind::Other, format!("{}",error))))
            };

            let (session_cookie, username_cookie) = get_login_cookies(session, sign_in.id);

            let response = ResponseBuilder::new()
                .set_status_code(StatusCode::Accepted)
                .set_html_content(contents)
                .add_cookie(session_cookie)
                .add_cookie(username_cookie)
                .build();
            
            match stream.write_all(response.as_bytes()) {
                Ok(_) => Ok(stream),
                Err(error) => Err(HandlerError::new(stream, error))
            }
        },
    }    
}

fn client_test_handler(_: &HttpRequest, tera: Arc<Tera>, mut stream: TcpStream) -> Result<TcpStream, HandlerError> {

    let mut context = tera::Context::new();
    context.insert("name", "Test clients");

    let contents = match tera.render("client_test.html", &context){
        Ok(contents) => contents,
        Err(error) => return Err(HandlerError::new(stream, Error::new(ErrorKind::Other, format!("{}",error))))
    };

    let response = ResponseBuilder::new()
        .set_status_code(StatusCode::Ok)
        .set_html_content(contents)
        .build();

    match stream.write_all(response.as_bytes()) {
        Ok(_) => Ok(stream),
        Err(error) => Err(HandlerError::new(stream, error))
    }
}

fn add_client_test_handler(_: &HttpRequest, tera: Arc<Tera>, mut stream: TcpStream) -> Result<TcpStream, HandlerError> {
    // acquire the html of the page.
    // let status_line = match req {
    //     HttpRequest::Get { status_line, .. } => status_line,
    //     HttpRequest::Post { status_line, .. } => status_line,
    // };


    let mut context = tera::Context::new();
    context.insert("src", "/puzzle/1");

    let contents = match tera.render("client_test_grid.html", &context){
        Ok(contents) => contents,
        Err(error) => return Err(HandlerError::new(stream, Error::new(ErrorKind::Other, format!("{}",error))))
    };

    let response = ResponseBuilder::new()
        .set_status_code(StatusCode::Ok)
        .set_html_content(contents)
        .build();

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

    let path_info = Regex::new(r"(?<num>\d+)").expect("Invalid regular expression");
    let caps = match path_info.captures(&status_line.route) {
        Some(caps) => caps,
        None => return Err(HandlerError::new(stream, Error::new(ErrorKind::Other, format!("api route doesn't match the regex used by the route handler"))))
    };
    let puzzle_num = caps["num"].parse::<i64>().unwrap();

    let mut context = tera::Context::new();
    context.insert("src", &format!("/puzzle/{puzzle_num}"));
    let data = match get_puzzle_db(&puzzle_num) {
        Ok(data) => data,
        Err( error) if error == rusqlite::Error::QueryReturnedNoRows => {
            return not_found(tera, stream, Some(&format!("No puzzle with ID {puzzle_num}")))
        },
        Err(error) => {
            return Err(HandlerError::new(stream, Error::new(ErrorKind::Other, format!("{}",error))))
        },
    };

    context.insert("name", &format!("{}",data.name));
    let contents = match tera.render("crossword.html", &context){
        Ok(contents) => contents,
        Err(error) => return Err(HandlerError::new(stream, Error::new(ErrorKind::Other, format!("{}",error))))
    };

    let response = ResponseBuilder::new()
        .set_status_code(StatusCode::Ok)
        .set_html_content(contents)
        .build();

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

    let path_info = Regex::new(r"(?<num>\d+)/data").expect("Invalid regular expression");
    let caps = match path_info.captures(&status_line.route) {
        Some(caps) => caps,
        None => return Err(HandlerError::new(stream, Error::new(ErrorKind::Other, format!("api route doesn't match the regex used by the route handler"))))
    };

    let puzzle_num = caps["num"].parse().unwrap();

    match PUZZLEPOOL.lock(){
        Ok(mut mut_guard) => return mut_guard.get_grid_data(puzzle_num , stream),
        Err(e) => return Err(HandlerError::new(stream, Error::new(ErrorKind::Other, format!("{}", e))))
    }
}

fn puzzle_handler_live(req: &HttpRequest, tera: Arc<Tera>, mut stream: TcpStream) -> Result<TcpStream, HandlerError> {

    let status_line = match req {
        HttpRequest::Get { status_line, .. } => status_line,
        HttpRequest::Post { status_line, .. } => status_line,
    };

    let path_info = Regex::new(r"(?<num>\d+)/live").expect("Invalid regular expression");
    let caps = match path_info.captures(&status_line.route) {
        Some(caps) => caps,
        None => return Err(HandlerError::new(stream, Error::new(ErrorKind::Other, format!("api route doesn't match the regex used by the route handler"))))
    };
    let puzzle_num = caps["num"].parse().unwrap();

    let handshake = match websocket_handshake(req){
        Ok(handshake) => handshake,
        Err(_) => return bad_request(tera, stream, "malformed handshake")
    };

    if let Err(error) = stream.write_all(handshake.as_bytes()) {
        return Err(HandlerError::new(stream, error))
    }


    match PUZZLEPOOL.lock(){
        Ok(mut mut_guard) => {
            match mut_guard.connect_client(puzzle_num, stream) {
                Ok(stream) => {
                    return Ok(stream)
                }
                Err(mut handler) => {
                    let data = close_websocket_message();
                    if let Err(e) = handler.stream.write_all(&data) {
                        error!("Could not write the the close handshake to the client");
                        return Err(HandlerError::new(handler.stream, e))
                    };
                    return Err(HandlerError::new(handler.stream, Error::from(ErrorKind::Other)))
                }
            }
        },
        Err(e) => return Err(HandlerError::new(stream, Error::new(ErrorKind::Other, format!("{}", e))))
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct AddPuzzleBody{
    name: String,
    crossword: Crossword
}

fn puzzle_add_handler(req: &HttpRequest, tera: Arc<Tera>, mut stream: TcpStream) -> Result<TcpStream, HandlerError> {

    let _status_line = match req {
        HttpRequest::Get {  .. } => return bad_request(tera, stream, "Unsupported http method"),
        HttpRequest::Post { status_line: _, headers: _, body} => {

            let body = match std::str::from_utf8(&body) {
                Ok(s) => s,
                Err(_) => {
                    return bad_request(tera, stream, "Body of the request was not valid UTF-8")
                },
            };

            let request_data: AddPuzzleBody  = match serde_json::from_str(body){
                Ok(s) => s,
                Err(e) => {
                    return bad_request(tera, stream, &format!("Body of the request did not match the schema for adding puzzles to the database {e}"))
                },
            };

            let id = match create_new_puzzle(&request_data.name, &request_data.crossword) {
                Ok(id) => id,
                Err(error) => return Err(HandlerError::new(stream, error))
            };


            let puzzle_info = get_puzzle_db(&id).unwrap();

            let contents = match serde_json::to_string(&puzzle_info){
                Ok(s) => s,
                Err(e) => {
                    return server_error(tera, stream)
                },
            };

            let response = ResponseBuilder::new()
                .set_status_code(StatusCode::Ok)
                .set_json_content(contents)
                .build();

            match stream.write_all(response.as_bytes()) {
                Ok(_) => return Ok(stream),
                Err(error) => return Err(HandlerError::new(stream, error))
            }

        },
    };

}

fn puzzle_list_handler(req: &HttpRequest, tera: Arc<Tera>, mut stream: TcpStream) -> Result<TcpStream, HandlerError> {
    let puzzle_data = match get_all_puzzle_db(){
        Ok(puzzle_data) => puzzle_data,
        Err(error) => return Err(HandlerError::new(stream, Error::new(ErrorKind::Other, format!("{}",error))))
    };

    let contents = match serde_json::to_string(&puzzle_data){
        Ok(s) => s,
        Err(error) => {
            error!("Unsuccessfully serialised puzzle data, {}", error);
            return server_error(tera, stream)
        },
    };

    let response = ResponseBuilder::new()
        .set_json_content(contents)
        .set_status_code(StatusCode::Ok)
        .build();

    match stream.write_all(response.as_bytes()) {
        Ok(_) => return Ok(stream),
        Err(error) => return Err(HandlerError::new(stream, error))
    }

}

#[derive(Debug)]
struct PuzzlePool {
    pool: HashMap<i64, Arc<Mutex<PuzzleChannel>>>,
    tera: Arc<Tera>
}

impl PuzzlePool {
    fn new() -> Self {
        let pool = HashMap::new();

        let tera = Tera::new("templates/**/*").unwrap_or_else(|err| {
            error!("Sever failed to load templates: {}", err);
            std::process::exit(1);
        });

        Self { pool, tera: Arc::new(tera) }
    }

    fn connect_client(&mut self, puzzle_num: i64, stream: TcpStream) -> Result<TcpStream, HandlerError> {


        match self.pool.get(&puzzle_num) {
            Some(puzzle_channel) => {
                info!("Connecting websocket client to existing puzzle.");
                route_stream_to_puzzle(puzzle_channel.clone(), stream, self.tera.clone())
            }
            None => {
                info!("No channel found to route websocket client. Creating a new channel");
                match PuzzleChannel::new(puzzle_num.clone()){
                    Ok(channel) => {
                        match channel {
                            Some(channel) => {
                                let new_channel = Arc::new(Mutex::new(channel));
                                self.pool.insert(puzzle_num, new_channel.clone());
                                route_stream_to_puzzle(new_channel.clone(), stream,  self.tera.clone())
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

    fn get_grid_data(&mut self, puzzle_num: i64, mut stream: TcpStream) -> Result<TcpStream, HandlerError> {
        self.pool.iter().for_each(|(name,_)|{
            info!("channel {}",name)
        });
        match self.pool.get(&puzzle_num) {
            Some(puzzle_channel) => {
                // get crossword from channel
                info!("Puzzle channel found. Sending puzzle channel data.");
                match puzzle_channel.lock() {
                    Ok(mut_guard) => return mut_guard.send_puzzle(stream),
                    Err(err) => {
                        error!("The puzzle channel thread has panicked: {err}");
                        match stream.try_clone() {
                            Ok(s) => return server_error( self.tera.clone(), s),
                            Err(error) => {
                                error!("Cannot write anything to the client as cloning the Tcp stream failed: {}",error);
                                return Err(HandlerError::new(stream, error));
                            }
                        }
                    },
                }
            }
            None => {
                info!("Puzzle channel not found. Loading data from disk");

                match get_puzzle(&puzzle_num) {
                    Ok(grid) => {

                        let contents = match serde_json::to_string(&grid){
                            Ok(s) => s,
                            Err(e) => {
                                return bad_request(self.tera.clone(), stream, &format!("The crossword did not match the schema expected by the database {e}")) 
                            },
                        };

                        let response = ResponseBuilder::new()
                        .set_status_code(StatusCode::Ok)
                        .set_json_content(contents)
                        .build();

                        match stream.write_all(response.as_bytes()) {
                            Ok(_) => return Ok(stream),
                            Err(error) => return Err(HandlerError::new(stream, error))
                        }
                    },
                    Err(e) => {
                        warn!("Cannot find puzzle: {e}");
                        return not_found(self.tera.clone(), stream, Some(&format!("Can't find puzzle {puzzle_num}")));        
                    }
                }
            }
        }
    }

    fn remove_channel(&mut self, puzzle_num: &i64) {
        self.pool.remove(puzzle_num);
    }
}

type ThreadSafeSenderVector = Arc<Mutex<Vec<Arc<Sender<Message>>>>>;

#[derive(Debug)]
struct PuzzleChannel {
    channel_wide_sender: Arc<Sender<Message>>,
    clients: ThreadSafeSenderVector,
    terminate_sender: mpsc::Sender<bool>,
    crossword: Arc<Mutex<Crossword>>,
    puzzle_num: i64,
    tera: Arc<Tera>
}

impl PuzzleChannel {
    fn new(puzzle_num: i64) -> Result<Option<Self>, Error> {
        // let puzzle_num_clone = puzzle_num.clone();

        let (sender, receiver) = mpsc::channel::<Message>();

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

                let msg = match receiver.recv() {
                    Ok(d) => d,
                    Err(e) => {
                        error!("There was an error recieving data: {e}");
                        break;
                    }
                };

                let msg_clone = msg.clone();
                
                match msg_clone.opcode {
                    OpCode::Continuation => todo!(),
                    OpCode::Text => send_msg_to_clients(&msg, &crossword_clone, puzzle_num),
                    OpCode::Binary => todo!(),
                    OpCode::Reserved(_) => todo!(),
                    OpCode::Close => trace!("Close"),
                    OpCode::Ping => trace!("Ping"),
                    OpCode::Pong => trace!("Pong"),
                }

                clients_clone
                    .lock()
                    .unwrap_or_else(|err| {
                        warn!("This mutex is in a poisoned state, but we're attempting to send the clients messages anyway");
                        err.into_inner()
                    })
                    .iter()
                    .filter_map(|x| x.send( msg.clone() ).err())
                    .for_each(drop);
                }
            
            info!("finishing");
            PUZZLEPOOL.lock().unwrap_or_else(|err| {
                warn!("This mutex is in a poisoned state, but we're attempting to remove the channel anyway");
                err.into_inner()
            }).remove_channel(&puzzle_num);

        })
        {
            Ok(_) => info!("Succesfully exceuted puzzle channel creation"),
            Err(e) => {
                info!("Failed to exceuted puzzle channel creation {0:?}", e);
                return Err(Error::new(ErrorKind::Other, format!("{:?}", e)))
            },
        }

        let tera = Tera::new("templates/**/*").unwrap_or_else(|err| {
            error!("Sever failed to load templates: {}", err);
            std::process::exit(1);
        });

        Ok(Some(Self {
            channel_wide_sender: Arc::new(sender),
            clients,
            terminate_sender,
            crossword,
            puzzle_num,
            tera: Arc::new(tera)
        }))
    }

    fn add_new_client(&mut self, sender: Arc<Sender<Message>>) {
        info!("adding new client to senders");
        self.clients.lock().unwrap_or_else(|err| {
            warn!("This mutex is in a poisoned state, but we're attempting to add a new client anyway");
            err.into_inner()
        }).push(sender)
    }

    fn remove_client(&mut self, sender: &Arc<Sender<Message>>) {
        let mut clients = self.clients.lock().unwrap_or_else(|err| {
            warn!("This mutex is in a poisoned state, but we're attempting to remove the client anyway");
            err.into_inner()
        });
        if let Some(idx) = clients.iter().position(|x| Arc::ptr_eq(x, sender)) {
            clients.remove(idx);
            info!("found client, removing")
        }

        info!("number of remaining clients: {}",clients.len());
        if clients.len() == 0 {
            info!("terminating channel");
            if let Err(e) = self.terminate_sender.send(true) {
                error!("There was an error broadcasting the termination signal: {e}")
            }
        }

    }


    fn send_puzzle(&self, mut stream: TcpStream) -> Result<TcpStream, HandlerError> {
        let grid = match self.crossword.lock() {
            Ok(grid) => grid,
            Err(e) => {
                error!("The crossword is in a poisoned state {e}");
                return server_error(self.tera.clone(), stream)
            }
        };

        let contents = match serde_json::to_string(&*grid){
            Ok(s) => s,
            Err(e) => {
                error!("The crossword could not be serialised to json {e}");
                return server_error(self.tera.clone(), stream)
            },
        };
        let response = ResponseBuilder::new()
        .set_status_code(StatusCode::Ok)
        .set_json_content(contents)
        .build();
        match stream.write_all(response.as_bytes()) {
            Ok(_) => Ok(stream),
            Err(error) => Err(HandlerError::new(stream, error))
        }
    }

}

fn send_msg_to_clients(msg: &Message, crossword_clone: &Arc<Mutex<Crossword>>, puzzle_num: i64) {
    match String::from_utf8(msg.clone().body) {
        Ok(client_data) => {
            info!("decoded incoming data: {}", client_data);
            let incoming_data: Result<Cell, serde_json::Error>  = serde_json::from_str(&client_data);
            match incoming_data {
                Ok(deserialised) => {
                    match crossword_clone.lock() {
                        Ok(mut guard) => guard.update_cell(deserialised),
                        Err(e)  => {
                            warn!("puzzle {} is poisoned, but we're sending the data anyway", puzzle_num);
                            e.into_inner().update_cell(deserialised)
                        }
                    }
                
                },
                Err(_) => {
                    warn!("cannot deserialise into cell data")
                },
            }
        
        },
        Err(e) => warn!("tried to decode, but there was an error: {}", e),
    }
}

impl Drop for PuzzleChannel {
    fn drop(&mut self) {

        let data = self.crossword.lock().unwrap_or_else(|e| {
            warn!("Crossword data may be corrupt");
            e.into_inner()
        })
        ;
        match save_puzzle(&self.puzzle_num ,&data) {
            Ok(_) => info!("dropping puzzle channel"),
            Err(e) => error!("Failed to save puzzle {e}")
        }
        
    }
}


fn route_stream_to_puzzle(puzzle_channel: Arc<Mutex<PuzzleChannel>>,stream: TcpStream, tera: Arc<Tera>) -> Result<TcpStream, HandlerError>{

    let stream_clone = match stream.try_clone(){
        Ok(stream) => stream,
        Err(e) => {
            error!("Could not route stream to puzzle as the stream could not be cloned: {e}");
            return server_error(tera, stream)
        }
    };

    let _ = stream.set_read_timeout(Some(Duration::from_millis(10)));
    
    let stream_arc = Arc::new(Mutex::new(stream));
    let (sender, receiver) = mpsc::channel::<Message>();

    let (terminate_sender, terminate_rec) = mpsc::channel();

    let sender = Arc::new(sender);
    let sender_clone = sender.clone();
    {
        match puzzle_channel.lock() {
            Ok(mut guard) => guard.add_new_client(sender),
            Err(e) => {
                error!("Could not add a new client to the puzzle channel as it is in a poisoned state: {e}");
                return server_error(tera, stream_clone)
            }
        }
    }

    let channel_wide_sender = match puzzle_channel.lock(){
        Ok(guard) => guard.channel_wide_sender.clone(),
        Err(e) => {
            error!("Could not add aquire the channel-wide sender as the puzzle channel is in a poisoned state: {e}");
            return server_error(tera, stream_clone)
        }
    };

    let heartbeat_channel_wide_sender = match puzzle_channel.lock(){
        Ok(guard) => guard.channel_wide_sender.clone(),
        Err(e) => {
            error!("Could not add aquire the channel-wide sender as the puzzle channel is in a poisoned state: {e}");
            return server_error(tera, stream_clone)
        }
    };

    match THREADPOOL.execute( move || {
        loop {            
            match heartbeat_channel_wide_sender.send(Message::ping_message()){
                Ok(_) => trace!("Server heart beat"),
                Err(_) => {
                    warn!("failed to send heart beat");
                    break
                },
            }
            sleep(Duration::from_millis(5000))
        };
        info!("Finished sending heart beats");
    }){
        Ok(_) => info!("Succesfully set up Heartbeats"),
        Err(error) => {
            info!("Failed to exceuted puzzle channel creation {0:?}", error);
            return Err(HandlerError::new(stream_clone, Error::new(ErrorKind::Other, format!("Failed to exceuted puzzle channel creation {0:?}", error))))
        },
    };

    let stream_writer = Arc::clone(&stream_arc);
    match THREADPOOL.execute( move || {
        loop {
            if let Ok(should_break) = terminate_rec.recv_timeout(Duration::from_millis(10)){
                if should_break {
                    trace!("ending here");
                    break
                }
                else {
                    trace!("not breaking here");
                }
            }
            let msg = match receiver.recv() {
                Ok(msg) => msg,
                Err(e) => {
                    warn!("The channel-wide sender is diconnected: {e} ");
                    break
                }
            };
            trace!("{:?}",msg);
            let frame: Vec<u8> = msg.into();

            let res = stream_writer.lock().unwrap_or_else(|e| {
                warn!("Acquired a lock on a poisoned stream. Sening message anyway. Error: {e}");
                e.into_inner()
            }).write_all(&frame);

            match res {
                Ok(_) => trace!("sent message"),
                Err(error) => {
                    match error.kind() {
                        ErrorKind::BrokenPipe => {
                            info!("Client disconnected");
                            break
                        },
                        _ => error!("There was an error writing to the the stream from this puzzle channel: {error}"),
                    }
                },
            }
        }
        info!("finished writing data to client");
        puzzle_channel.lock().unwrap_or_else(|e| {
            warn!("Acquired a lock on a poisoned puzzle channel. deleting client anyway. Error: {e}");
            e.into_inner()
        }).remove_client(&sender_clone);

    }) {
        Ok(_) => info!("Succesfully set up receiver"),
        Err(error) => {
            info!("Failed to exceuted puzzle channel creation {0:?}", error);
            return Err(HandlerError::new(stream_clone, Error::new(ErrorKind::Other, format!("Failed to exceuted puzzle channel creation {0:?}", error))))
        },
    }

    match THREADPOOL.execute(move || {
        loop {
            {
                let mut guard = match stream_arc.lock() {
                    Ok(guard) => guard,
                    Err(e) => {
                        error!("Send thread is poisoned: {e}. Ending websocket stream");
                        if let Err(e) = terminate_sender.send(true) {
                            error!("There was an error broadcasting the termination signal: {e}")
                        }
                        break
                    }
                };

                let mut buf_reader = BufReader::new(&mut *guard);
                let _ = match decode_client_frame(&mut buf_reader) {
                    Ok(msg) => {
                        // put here!
                        match msg.opcode {
                            OpCode::Continuation => todo!(),
                            OpCode::Ping => {trace!("Ping"); Ok(())},
                            OpCode::Pong => {trace!("Pong"); Ok(())},
                            OpCode::Close => {
                                info!("Received close message");
                                // if let Err(e) = channel_wide_sender.send(msg) {
                                //     error!("There was an error broadcasting the message to via the channel wide sender: {e}")
                                // }

                                let frame = close_websocket_message();
                                if let Err(e) = guard.write_all(&frame) {
                                    error!("Could not write the closing websocket message to the client: {e}")
                                } 
                                if let Err(e) = terminate_sender.send(true){
                                    error!("There was an error broadcasting the message to via the channel wide sender: {e}")
                                }
                                break
                            },
                            OpCode::Reserved(x) => {
                                warn!("Cannot handle op code {}", x);
                                Ok(())
                            },
                            OpCode::Text => channel_wide_sender.send(msg),
                            OpCode::Binary => channel_wide_sender.send(msg),
                        }
                    },
                    Err(_err) => {
                        Ok(())
                    },
                };
            }
            sleep(Duration::from_millis(10))
        }
        // puzzle_channel;
        info!("finished reading websocket from client");


    }) {
        Ok(_) => info!("Succesfully set up receiver"),
        Err(error) => {
            info!("Failed to exceuted puzzle channel creation {0:?}", error);
            return Err(HandlerError::new(stream_clone, Error::new(ErrorKind::Other, format!("Failed to exceuted puzzle channel creation {0:?}", error))))
        },
    }


    info!("Routed client to puzzle");
    Ok(stream_clone)
}