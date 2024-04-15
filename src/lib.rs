pub mod crossword;
pub mod db;
pub mod websockets;

use std::{
    collections::HashMap, fmt, io::{prelude::*, BufReader, Error, ErrorKind}, net::TcpStream, sync::{mpsc::{self, RecvError, SendError}, Arc, Mutex}, thread
};
use log::{error, info, trace, warn};

type Job = Box<dyn FnOnce() + Send + 'static>;

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
            .map(|(key,val)| format!("{:15}: {}", key, val))
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

#[allow(dead_code)]
pub struct ThreadPool{
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

#[allow(dead_code)]
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

            // job()
            // let mutex_guard = match receiver.lock() {
            //     Ok(l) => l,
            //     Err(e) => {
            //         error!("Failed to aquire a lock on worker {0}. Error: {1}", id, e);
            //         continue;
            //     }
            // };

            // match mutex_guard.recv() {
            //     Ok(job) => {
            //         trace!("Worker {} received Job, executing", id);
            //         job();
            //         trace!("Worker {} finished Job, freeing", id);
            //     },
            //     Err(e) => {
            //         error!("The Threadpool's sender has disconnected: {0}. No more messages can ever be sent to the thread pool", e);
            //         break;
            //     }
            // };
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

#[allow(dead_code)]
enum Content {
    ApplicationOctetStream {content: Vec::<u8>},
    TextPlain {content: String},
    TextCss {content: String},
    TextHtml {content: String},
    TextJavascript {content: String}
}

#[allow(dead_code)]
pub struct Response {
    status_line: String,
    headers: HashMap<String,String>,
    content: Content
}

pub trait Writable {
    fn write(&self) -> Vec::<u8>;
}

impl Response  {

    #[allow(dead_code)]
    fn new<T>(content: T, _extra_headers: Option<HashMap<String,String>>) -> Self where T: ExactSizeIterator{
        let status_line = "HTTP/1.1 200 Ok";

        let length = content.len();
        let headers: HashMap<String, String> = HashMap::from([("Content-Length".to_string(), format!("{length}"))]);

        let c = Content::TextCss { content: "123".to_owned() };
        Response {status_line: status_line.to_string(), headers, content: c }
    }

}
