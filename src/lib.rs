#![feature(iterator_try_collect)]
#![feature(let_chains)]

pub mod crossword;
pub mod db;
pub mod websockets;
pub mod response;

use std::{
    collections::HashMap, fmt, io::{prelude::*, BufReader, Error, ErrorKind}, net::TcpStream, sync::{mpsc::{self}, Arc, Mutex}, thread
};
use log::{error, info, trace, warn};
use response::SetCookie;

type Job = Box<dyn FnOnce() + Send + 'static>;

pub fn get_form_data(raw_form: &str) -> Result<HashMap<&str,Option<&str>>,Error> {

    if raw_form == "" {
        return Err(Error::new(ErrorKind::InvalidData, "Empty form data"))
    } 

    let form_data: HashMap<&str,Option<&str>> = raw_form
        .split("&")
        // .map_while(|x| x.contains("="))
        .map(| x|{
            if x.contains("=") {
                let kv: Vec<&str> = x.split("=").collect();
                let k = *kv.get(0).unwrap();
                let v = kv.get(1).and_then(|x| match *x {
                    "" => None,
                    x => Some(x)
                });
                if k == "" {
                    error!("Field missing name");
                    return Err(Error::new(ErrorKind::InvalidData, "field should have a name"))
                } else {
                    Ok((k,v))
                }
            }
            else {
                error!("Field missing =");
                Err(Error::new(ErrorKind::InvalidData, "field should have an = to be parsed"))
            }
        }).try_collect()?;
        
        Ok(form_data)
}


#[derive(Debug)]
pub enum HttpRequest {
    // GET{headers: Vec<String>},
    // POST{headers: Vec<String>, body: String}
    Get{ status_line: StatusLine, headers: HashMap<String,String> },
    Post{ status_line: StatusLine, headers: HashMap<String,String>, body: Vec<u8> }
}

#[derive(Debug, Clone, Copy)]
pub enum HttpVerb {
    Get,
    Post
}

impl HttpVerb {
    fn new(s: &str) -> Result<Self, String> {
        match s {
            "GET" => Ok(HttpVerb::Get),
            "POST" => Ok(HttpVerb::Post),
            s=> Err(format!("Unknown Method: {s}"))
        }
    }
}

impl fmt::Display for HttpVerb {

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpVerb::Get => write!(f,"GET"),
            HttpVerb::Post => write!(f,"POST"),
        }
    }

}


#[derive(Debug)]
pub struct StatusLine {
    pub protocol: String,
    verb: HttpVerb,
    pub route: String
}

impl StatusLine {
    fn new(status_line: &str) -> Result<Self, Error> {
        trace!("Creating status line from '{}'",status_line);
        let parts: Vec<_> = status_line.split(" ").collect();

        let verb = match HttpVerb::new(parts[0]){
            Ok(val) => val,
            Err(_err) => {
                warn!("status line:{}. Cannot process this request", status_line);
                return Err(Error::from(ErrorKind::InvalidData))
            },
        };
        let protocol = parts[2].trim().to_string();
        let route = parts[1].to_string();
        Ok(StatusLine{ protocol: protocol, verb, route: route })

    }
}

impl fmt::Display for StatusLine {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {} {}", self.verb, self.route, self.protocol)
    }
}

impl HttpRequest {
    pub fn new(mut stream: &TcpStream) -> Result<Self, Error> {
        let mut buf_reader = BufReader::new(&mut stream);
        let mut start_line = String::new();

        buf_reader.read_line(&mut start_line)?;
        
        let status_line = StatusLine::new(&start_line)?;
    
        match status_line.verb {
            HttpVerb::Get  => {
                match Self::process_headers(&mut buf_reader) {
                    Ok(headers) => Ok( Self::Get{status_line, headers} ),
                    Err(e) => Err(e)
                }
            },
            HttpVerb::Post  => {
                let headers = Self::process_headers(&mut buf_reader)?;
                let len = headers["Content-Length"].parse::<usize>().map_err(|err| Error::new(ErrorKind::InvalidData, err))?;
                let mut buf = vec![0; len];
                let _ = buf_reader.read_exact(&mut buf);
                Ok( Self::Post{status_line, headers, body:buf} )
            }
        }
    }
   
    fn process_headers(req: &mut BufReader<&mut &TcpStream>) -> Result<HashMap<String,String>, Error>{
        let headers = req
            .lines()
            .take_while(|res| match res {
                Ok(line) => !line.is_empty(),
                Err(_) => false
            })
            .map(|line_result| {
                let line = line_result?;
                let mut s = line.splitn(2,": ");
                let first = s.next().ok_or_else(|| Error::new(ErrorKind::InvalidData, format!("Attempted to convert the line {line} header did not have a first part. Attempted to split at the `:` character")))?.to_string();
                let second = s.next().ok_or_else(|| Error::new(ErrorKind::InvalidData, format!("Attempted to convert the line {line} header did not have a second part. Attempted to split at the `:` character")))?.to_string();
                Ok((first,second))
            })
            .collect::<Result<HashMap<String, String>, Error>>()?;

        return Ok(headers);
    }
}

impl fmt::Display for HttpRequest {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

        fn format_headers(headers: &HashMap<String,String>) -> String {
            let header_summary = headers
            .into_iter()
            .map(|(key,val)| format!("{}: {}", key, val))
            .collect::<Vec<String>>()
            .join("\n");
            return header_summary;
        }


        match self {
            HttpRequest::Get { status_line, headers } => {

                let header_summary = format_headers(headers);

                write!(f, "Request:\n{status_line}\n{header_summary}")

            },
            HttpRequest::Post { status_line, headers, body: _ } => {

                let header_summary = format_headers(headers);

                write!(f, "Request:\n{status_line}\n{header_summary}")
            },
        }
        
    }
}

pub fn get_login_cookies(session: i64, username: i64) -> (SetCookie<String>, SetCookie<String>) {
    let mut session_cookie = SetCookie::new("session-id".to_string(), session.to_string());
    session_cookie.set_max_age(chrono::Duration::minutes(60));

    let mut username_cookie = SetCookie::new("user-id".to_string(), username.to_string());
    username_cookie.set_max_age(chrono::Duration::minutes(60));
    (session_cookie, username_cookie)
}

pub fn is_authorised(headers: &HashMap<String,String>) -> Result<(),String> {

    #[derive(PartialEq, Eq)]
    struct Cookie<'a> {
        name: &'a str,
        value: &'a str
    }

    let cookies = headers.get("Cookie");
    if cookies.is_none() {
        info!("Missing cookie header");
        return Err("missing session-id or user-id".to_string())       
    }
    let cookies: Vec<Cookie> = cookies.unwrap().split(";").map(|x| {
        let mut c = x.split("=");
        let name = c.next().unwrap().trim();
        let value = c.next().unwrap().trim();
        Cookie{ name, value}
    }).collect();

    let session_cookie: Option<&Cookie> = cookies.iter().find(|x| x.name=="session-id");
    let user_id_cookie: Option<&Cookie> = cookies.iter().find(|x| x.name=="user-id");

    let session: i64 = match session_cookie {
        Some(c) => {
            match c.value.parse() {
                Ok(x) => x,
                Err(e) => {
                    info!("Can't parse session cookie as an int");
                    return Err(format!("Session must be an integer - found {}", e))
                }
            }
        },
        None => return Err("missing session-id".to_string()),
    };

    let id: i64 = match user_id_cookie {
        Some(c) => {
            match c.value.parse() {
                Ok(x) => x,
                Err(e) => {
                    info!("Can't parse user-id cookie as an int");
                    return Err(format!("Session must be an integer - found {}", e))
                }
            }
        },
        None => return Err("missing user-id".to_string()),
    };


    match db::check_session(id,session) {
        Ok(_) => return {
            trace!("User signed in");
            Ok(())
        },
        Err(_) => return Err("Failed to validate user session".to_string()),
    }
}

pub struct ThreadPool{
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>
    // receiver: mpsc::Receiver<Job>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>)-> Worker {
        let thread = thread::spawn( move || loop {


            let recv_result = receiver.lock().unwrap_or_else(|e| {
                warn!("Acquired a lock on the receiver mutex, but the last user of this mutex left it in a a poisoned state.");
                e.into_inner()
            } ).recv();

            match recv_result  {
                Ok(job) => job(),
                Err(e) => {
                    error!("There is no reciever anymore, so worker thread will begin ending. Full error: {e}");
                    break;
                },
            }
        });

        trace!("Creating worker: {}",id);
        Worker { id, thread : Some(thread) }
    }
}

#[derive(Debug)]
pub enum ThreadPoolError {
    NoReceiver,
    NoSender,
}

impl ThreadPool {

    pub fn new(size: usize)-> ThreadPool{
        assert!(size > 0);
        info!("Creating a threadpool with {size} workers.");

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(Worker::new(id,Arc::clone(&receiver)));
        }

        ThreadPool {workers, sender: Some(sender)}
    }

        pub fn execute<F>(&self, f: F)-> Result<(),ThreadPoolError> where F: FnOnce() + Send + 'static {
            let job = Box::new(f);

            match self.sender.as_ref() {
                Some(sender) => {
                    match sender.send(job) {
                        Ok(_) => return Ok(()),
                        Err(_) => {
                            error!("The reciever for this sender has been deallocated.");
                            return Err(ThreadPoolError::NoReceiver)
                        }
                    }
                },
                None => {
                    error!("Could not send job to any workers because the threadpool's sender is no more.");
                    return Err(ThreadPoolError::NoSender)
                }
            };  

        }
    
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                if let Err(_) = thread.join() {
                    warn!("The threadpool was dropped, but one of the workers paniced while joining.");
                }
            }

        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, io::ErrorKind};

    use crate::get_form_data;


    #[test]
    fn test_empty_form_data() {
        let x = get_form_data("");
        assert_eq!(x.map_err(|e| e.kind()),Err(ErrorKind::InvalidData))
    }

    #[test]
    fn test_one_form_data() {
        let x = get_form_data("a=b");
        assert_eq!(x.unwrap(), HashMap::from([("a",Some("b"))]))
    }

    #[test]
    fn test_two_form_data() {
        let x = get_form_data("a=b&c=d");
        assert_eq!(x.unwrap(), HashMap::from([("a",Some("b")),("c",Some("d"))]))
    }

    #[test]
    fn test_simple_one_blank_field() {
        let x = get_form_data("a=");
        assert_eq!(x.unwrap(), HashMap::from([("a",None)]))
    }

    #[test]
    fn test_second_field_is_blank() {
        let x = get_form_data("a=b&c=");
        assert_eq!(x.unwrap(), HashMap::from([("a",Some("b")),("c",None)]))
    }

    #[test]
    fn test_middle_field_is_blank() {
        let x = get_form_data("a=b&c=&d=e");
        assert_eq!(x.unwrap(), HashMap::from([("a",Some("b")),("c",None),("d",Some("e"))]))
    }

    #[test]
    fn test_bad_field() {
        let x = get_form_data("a=b&c&d=e");
        assert_eq!(x.map_err(|e| e.kind()),Err(ErrorKind::InvalidData))
    }

    #[test]   
    fn test_lots_of_ampersands_field() {
        let x = get_form_data("&&&&&");
        assert_eq!(x.map_err(|e| e.kind()),Err(ErrorKind::InvalidData))
    }

    #[test]   
    fn test_field_with_no_name() {
        let x = get_form_data("=c");
        assert_eq!(x.map_err(|e| e.kind()),Err(ErrorKind::InvalidData))
    }
}