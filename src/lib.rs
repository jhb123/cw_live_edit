use std::{
    collections::HashMap,
    net::TcpStream, 
    io::{prelude::*, BufReader},
};

#[derive(Debug)]
pub enum HttpRequest {
    // GET{headers: Vec<String>},
    // POST{headers: Vec<String>, body: String}
    Get{ status_line: StatusLine, headers: HashMap<String,String> },
    Post{ status_line: StatusLine, headers: HashMap<String,String>, body: Vec<u8> }
}

#[derive(Debug)]
pub enum HttpVerb {
    Get,
    Post
}

impl HttpVerb {
    fn new(s: &str) -> Result<Self, &str> {
        match s {
            "GET" => Ok(HttpVerb::Get),
            "POST" => Ok(HttpVerb::Post),
            _  => Err("Unknown Method")
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
    fn new(status_line: &str) -> Self {
        let parts: Vec<_> = status_line.split(" ").collect();
        let verb = HttpVerb::new(parts[0]).unwrap();
        let protocol = parts[2].trim().to_string();
        let route = parts[1].to_string();
        StatusLine{ protocol, verb, route }

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
        
        let status_line = StatusLine::new(&start_line);
    
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
