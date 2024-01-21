use std::{
    collections::HashMap,
    net::TcpStream, 
    io::{prelude::*, BufReader},
};

#[derive(Debug)]
pub enum HttpRequest {
    // GET{headers: Vec<String>},
    // POST{headers: Vec<String>, body: String}
    GET{ headers: HashMap<String,String> },
    POST{ headers: HashMap<String,String>, body: Vec<u8> }
}

impl HttpRequest {
    pub fn new(mut stream: TcpStream) -> Result<Self, String> {
        let mut buf_reader = BufReader::new(&mut stream);
        // .lines()
        // .map(|result| result.unwrap());
        let mut start_line = String::new();

        let size = match buf_reader.read_line(&mut start_line) {
            Ok(line) => line,
            Err(..) => return Err("Failed to read start line".to_string()),
        };
        
        let status_line: Vec<_> = start_line.split(" ").collect();
    
        match status_line[0] {
            "GET"  => {
                let headers = Self::process_headers(&mut buf_reader);
                Ok( Self::GET{headers} )
            },
            "POST"  => {
                let headers = Self::process_headers(&mut buf_reader);
                let len = headers["Content-Length"].parse::<usize>().unwrap();
                let mut buf = vec![0; len];
                let _ = buf_reader.read_exact(&mut buf);
                Ok( Self::POST{headers, body:buf} )
            }
            x => Err(format!("Status line error {x}"))
        }
    }

    fn process_headers(req: &mut BufReader<&mut TcpStream>) -> HashMap<String,String> {
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
