use cw_grid_server::{response::{internal_error_response, ResponseBuilder, StatusCode}, websockets::{close_websocket_message, decode_client_frame, websocket_handshake, OpCode}, HttpRequest, ThreadPool
};
use lazy_static::lazy_static;
use log::{error, info, trace, warn};
use regex::Regex;
use std::{
    collections::HashMap, env, io::{prelude::*, BufReader, Error}, net::{TcpListener, TcpStream}, sync::Arc};

lazy_static! {
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


type HandlerFn = fn(&HttpRequest, TcpStream) -> Result<TcpStream, HandlerError>;
type RouteMapping = HashMap<&'static str, HandlerFn>;

fn main() {
    env_logger::init();

    let mut routes: RouteMapping = HashMap::new();

    routes.insert(r"^/echo$", puzzle_handler_live);

    let api: Api = Api::register_routes(routes);
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

fn puzzle_handler_live(req: &HttpRequest, mut stream: TcpStream) -> Result<TcpStream, HandlerError> {

    let handshake = match websocket_handshake(req){
        Ok(handshake) => handshake,
        Err(_) => return bad_request(stream)
    };

    if let Err(error) = stream.write_all(handshake.as_bytes()) {
        return Err(HandlerError::new(stream, error))
    }
    let mut stream_clone = stream.try_clone().unwrap();
    let mut buf_reader = BufReader::new(&mut stream_clone);

    loop {
        {

            let _ = match decode_client_frame(&mut buf_reader) {
                Ok(msg) => {
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
                            if let Err(e) = stream.write_all(&frame) {
                                error!("Could not write the closing websocket message to the client: {e}")
                            } 
                            break
                        },
                        OpCode::Reserved(x) => {
                            warn!("Cannot handle op code {}", x);
                            Ok(())
                        },
                        OpCode::Text => {
                            let frame: Vec<u8> = msg.into();
                            stream.write_all(&frame)
                        },
                        OpCode::Binary => {
                            let frame: Vec<u8> = msg.into();
                            stream.write_all(&frame)
                        },
                    }
                },
                Err(_err) => {
                    Ok(())
                },
            };
        }
    }
    return Ok(stream)

}


#[derive(Clone)]
struct Api {
    routes: RouteMapping,
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
                
                if let Err(err) = handler(req, stream) {
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

        if let Err(err) = not_found(stream) {
            error!("No routes were found, but the missing route handler threw an error: {}", err.error);
            if let Err(e) = self.server_error(err.stream) {
                warn!("Failed to send the client the server error page: {}", e.error);
            };
            return;
        }
    }
    
    fn register_routes(routes: RouteMapping ) -> Self {
        Self { routes }
    }

    fn bad_request(&self, stream: TcpStream) -> Result<TcpStream, HandlerError> {
        let r = bad_request(stream);
        trace!("Handled bad request");
        return r
    }

    fn server_error(&self, stream: TcpStream) -> Result<TcpStream, HandlerError> {
        return server_error( stream)
    }

}

fn not_found(mut stream: TcpStream) -> Result<TcpStream, HandlerError> {

    let contents = "Not found";
    let response = ResponseBuilder::new()
        .set_status_code(StatusCode::BadRequest)
        .set_html_content(contents.to_owned())
        .build();

    match stream.write_all(response.as_bytes()) {
        Ok(_) => Ok(stream),
        Err(error) => Err(HandlerError::new(stream, error))
    }
}

fn bad_request(mut stream: TcpStream) -> Result<TcpStream, HandlerError> {

    let contents = "Bad request";

    let response = ResponseBuilder::new()
        .set_status_code(StatusCode::BadRequest)
        .set_html_content(contents.to_owned())
        .build();

    match stream.write_all(response.as_bytes()) {
        Ok(_) => Ok(stream),
        Err(error) => Err(HandlerError::new(stream, error))
    }
}

fn server_error(mut stream: TcpStream) -> Result<TcpStream, HandlerError> {
    let contents = "Internal Server Error";
    let response = internal_error_response(&contents);
    match stream.write_all(response.as_bytes()) {
        Ok(_) => Ok(stream),
        Err(error) => Err(HandlerError::new(stream, error))
    }
}