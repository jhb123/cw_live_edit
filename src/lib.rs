use std::{
    collections::HashMap, fmt, io::{self, prelude::*, BufReader}, net::TcpStream, sync::{mpsc, Arc, Mutex}, thread
};
use base64::{Engine as _, engine::general_purpose};
use crypto::{digest::Digest, sha1::Sha1};
use log::{info, warn};

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
    fn new(status_line: &str) -> Result<Self, &str> {
        let parts: Vec<_> = status_line.split(" ").collect();

        let verb = match HttpVerb::new(parts[0]){
            Ok(val) => val,
            Err(_err) => {
                warn!("status line:{}. Cannot process this request", status_line);
                return Err(status_line)
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
    pub fn new(mut stream: &TcpStream) -> Result<Self, String> {
        let mut buf_reader = BufReader::new(&mut stream);
        // .lines()
        // .map(|result| result.unwrap());
        let mut start_line = String::new();

        let _ = match buf_reader.read_line(&mut start_line) {
            Ok(line) => line,
            Err(..) => return Err("Failed to read start line".to_string()),
        };
        
        let status_line = StatusLine::new(&start_line)?;
    
        match status_line.verb {
            HttpVerb::Get  => {
                let headers = Self::process_headers(&mut buf_reader);
                Ok( Self::Get{status_line, headers} )
            },
            HttpVerb::Post  => {
                let headers = Self::process_headers(&mut buf_reader);
                let len = headers["Content-Length"].parse::<usize>().unwrap();
                let mut buf = vec![0; len];
                let _ = buf_reader.read_exact(&mut buf);
                Ok( Self::Post{status_line, headers, body:buf} )
            }
        }
    }
   
    fn process_headers(req: &mut BufReader<&mut &TcpStream>) -> HashMap<String,String> {
        let headers = req
            .lines()
            .map(|result| result.unwrap())
            .take_while(|line| !line.is_empty())
            .collect::<Vec<String>>()
            .into_iter()
            .map(|x: String| {
                let mut s = x.splitn(2,": ");
                let first = s.next().unwrap().to_string();
                let second = s.next().unwrap().to_string();
                (first,second)
            })
            .collect();
        return headers;
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
    sender: mpsc::Sender<Job>,
}

#[allow(dead_code)]
struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>
    // receiver: mpsc::Receiver<Job>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;


impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>)-> Worker{
        let thread = thread::spawn( move || loop {
            let job = receiver.lock().unwrap().recv().unwrap();
            info!("Worker {} received Job, executing", id);
            job();
        });

        info!("Creating worker: {}",id);
        Worker { id, thread }
    }
}

impl ThreadPool {

    pub fn new(size: usize)-> ThreadPool{
        assert!(size > 0);
        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(Worker::new(id,Arc::clone(&receiver)));
        }

        ThreadPool {workers, sender}
    }

        pub fn execute<F>(&self, f: F) where F: FnOnce() + Send + 'static {
            let job = Box::new(f);

            self.sender.send(job).unwrap();    
    }


    // pub fn execute<F, T>(&self, f: F) -> JoinHandle<T>  
    //     where 
    //     F: FnOnce() -> T,
    //     F: Send + 'static,
    //     T: Send +'static {

    // }

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

pub fn web_socket_accept(sender_key: &str) -> String {
    let magic_str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let full_str = sender_key.to_owned()+magic_str;
    let mut hasher = Sha1::new();

    hasher.input_str(&full_str);

    let bytes_num = hasher.output_bytes();

    let mut buf = vec![0;bytes_num];
    hasher.result(&mut buf);
    general_purpose::STANDARD.encode(buf)
}

pub fn websocket_message(msg: &str) -> Vec<u8>{
    let mut payload: Vec<u8> = Vec::new();

    // FIN bit: 1 (final fragment)
    payload.push(0b1000_0001); // FIN bit: 1, Opcode: 1 (text frame)

    // Payload length: 13 (length of "Hello, WebSocket!")
    let len = msg.len() as u8;
    payload.push(len);

    // Payload data: "Hello, WebSocket!"
    payload.extend_from_slice(msg.as_bytes());

    for byte in &payload {
        print!("{:02X} ", *byte);
    }
    // info!("{}",bits_string);
    payload
}


pub fn decode_client_frame(buf_reader : &mut BufReader<&mut TcpStream>) -> io::Result<Vec<u8>> {

    // see  RFC 6455: https://www.rfc-editor.org/rfc/rfc6455.html#section-5.3
    let mut frame_header = vec![0; 2];   
    buf_reader.read_exact(&mut frame_header)?;
    let mut payload_len = (frame_header[1] & 0b0111_1111) as u64;
    let mut masking_key = vec![0;4];

    info!("7 bit payload len: {}", payload_len);
    match payload_len {
        0..=125 => {
            frame_header = vec![0; 4];
            buf_reader.read_exact(&mut frame_header).unwrap();
            masking_key = frame_header.try_into().unwrap();
        }
        126 => {
            frame_header = vec![0; 6];
            buf_reader.read_exact(&mut frame_header).unwrap();
            payload_len = websocket_content_len(&frame_header[0..2]).unwrap();
            masking_key = frame_header[2..].try_into().unwrap();

        }
        127 => {
            frame_header = vec![0; 12];
            buf_reader.read_exact(&mut frame_header).unwrap();
            payload_len = websocket_content_len(&frame_header[0..8]).unwrap();
            masking_key = frame_header[8..].try_into().unwrap();
        }
        128..=u64::MAX => {
            // impossible? 
        }
    }

    info!("full payload len: {}", payload_len);
    info!("masking key: {:02X?}", masking_key);

    let mut rec_msg = vec![0; payload_len as usize];
    buf_reader.read_exact(&mut rec_msg).unwrap();

    
    let decoded: Vec<u8> = rec_msg
        .iter()
        .enumerate()
        .map(|(index, el)| el^masking_key[index%4] )
        .collect();
    
    
    return Ok(decoded);

}

fn websocket_content_len(data: &[u8]) -> Result<u64, &str>{
    let num_shifts = data.len();
    match num_shifts {
        0..=8 => {
            Ok(data.iter()
                .rev()
                .enumerate()
                .map(|(idx, el)| (*el as u64) << 8*(idx))
                .fold(0,|acc, x| acc | x )
        )
        },
        _ => Err("array too long.")
    }
    // if num_shifts
    // 
}

#[cfg(test)]
mod tests {

    #[test]
    fn web_socket_accept_header() {
        let actual = crate::web_socket_accept("dGhlIHNhbXBsZSBub25jZQ==");
        assert_eq!(actual, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }

}