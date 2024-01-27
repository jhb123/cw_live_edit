use std::{
    io::prelude::*,
    net::{TcpListener, TcpStream}, collections::HashMap, thread, time::Duration, sync::Arc,
};
use cw_grid_server::{HttpRequest, ThreadPool};
use tera::Tera;

fn main() {

    let mut routes: HashMap<&'static str, fn(&HttpRequest,  Arc<Tera>) -> String> = HashMap::new();
    routes.insert("/", index_handler);
    routes.insert("/hello", hello_handler);
    
    let tera = Tera::new("templates/**/*").unwrap();
    let tera_arc = Arc::new(tera);
    
    let api: Api = Api::register_routes(routes, tera_arc);
    let api_arc = Arc::new(api);

    let pool = ThreadPool::new(4);

    let listener = TcpListener::bind("127.0.0.1:5051").unwrap();

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

#[derive(Clone)]
struct Api {
    routes: HashMap<&'static str, fn(&HttpRequest,  Arc<Tera>) -> String>,
    tera:  Arc<Tera>
}

impl Api{

    fn handle_request(&self, req: &HttpRequest) -> String{
        println!("{:?}",req);
        match req {
            HttpRequest::Get { status_line, headers: _ } => {
                let handler = self.routes.get(&status_line.route as &str).unwrap();
                handler(req, Arc::clone(&self.tera))
            },
            HttpRequest::Post { status_line, headers: _, body: _ } => {
                let handler = self.routes.get(&status_line.route as &str).unwrap();
                handler(req, Arc::clone(&self.tera))
            },
        }

    }

    fn register_routes(routes:  HashMap<&'static str, fn(&HttpRequest,  Arc<Tera>) -> String>, tera:  Arc<Tera> ) -> Self {
        Self{routes, tera}
    }

}

fn hello_handler(_req: &HttpRequest, tera:  Arc<Tera>) -> String{
    println!("hello route");
    thread::sleep(Duration::from_secs(5));
    let status_line = "HTTP/1.1 200 Ok";
    let mut context = tera::Context::new();
    context.insert("data", "Hello");
    let contents = tera.render("hello.html", &context).unwrap();
    let length = contents.len();
    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
    return response
}

fn index_handler(_req: &HttpRequest, tera: Arc<Tera>) -> String{
    println!("hello route");
    let status_line = "HTTP/1.1 200 Ok";
    let mut context = tera::Context::new();
    context.insert("data", "Index");
    let contents = tera.render("hello.html", &context).unwrap();
    let length = contents.len();
    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
    return response
}