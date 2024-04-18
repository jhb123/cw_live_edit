use std::{
    io::{self, BufReader, Error, ErrorKind, Read},
    net::TcpStream,
};

use base64::{engine::general_purpose, Engine as _};
use crypto::{digest::Digest, sha1::Sha1};
use log::{error, trace};

use crate::HttpRequest;

#[derive(Debug)]
pub enum OpCode {
    Continuation,
    Ping,
    Pong,
    Close,
    Reserved,
    Text,
    Binary,
}

pub struct Message {
    pub opcode: OpCode,
    pub body: Vec<u8>,
}

impl Message {
    fn new(opcode: OpCode, body: Vec<u8>) -> Self {
        Self { opcode, body }
    }
}

pub fn web_socket_accept(sender_key: &str) -> String {
    let magic_str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let full_str = sender_key.to_owned() + magic_str;
    let mut hasher = Sha1::new();

    hasher.input_str(&full_str);

    let bytes_num = hasher.output_bytes();

    let mut buf = vec![0; bytes_num];
    hasher.result(&mut buf);
    general_purpose::STANDARD.encode(buf)
}

pub fn websocket_message(msg: &str) -> Vec<u8> {
    let mut payload: Vec<u8> = Vec::new();

    payload.push(0b1000_0001); // FIN bit: 1, Opcode: 1 (text frame)

    let len = msg.len() as u8;
    payload.push(len);

    payload.extend_from_slice(msg.as_bytes());

    for byte in &payload {
        print!("{:02X} ", *byte);
    }
    payload
}

pub fn close_websocket_message() -> Vec<u8> {
    vec![0x88, 0x02, 0x03, 0xE8]
}

pub fn decode_client_frame(buf_reader: &mut BufReader<&mut TcpStream>) -> io::Result<Message> {
    // see  RFC 6455: https://www.rfc-editor.org/rfc/rfc6455.html#section-5.3
    let mut frame_header = vec![0; 2];
    buf_reader.read_exact(&mut frame_header)?;
    // opt codes
    let opcode = (frame_header[0] & 0b0000_1111) as u8;
    let opcode = match opcode {
        0x0 => OpCode::Continuation,
        0x1 => OpCode::Text,
        0x2 => OpCode::Binary,
        0x3..=0x7 => OpCode::Reserved,
        0x8 => OpCode::Close,
        0x9 => OpCode::Ping,
        0xA => OpCode::Pong,
        0xB..=0xF => OpCode::Reserved,
        _ => {
            return Err(Error::new(
                io::ErrorKind::InvalidInput,
                format!("client send unexpected opcode: {opcode}"),
            ))
        }
    };
    trace!("OpCode: {:?}", opcode);

    let mut payload_len = (frame_header[1] & 0b0111_1111) as u64;
    let mut masking_key = vec![0; 4];

    trace!("7 bit payload len: {}", payload_len);
    match payload_len {
        0..=125 => {
            frame_header = vec![0; 4];
            buf_reader.read_exact(&mut frame_header)?;
            masking_key = frame_header;
        }
        126 => {
            frame_header = vec![0; 6];
            buf_reader.read_exact(&mut frame_header)?;
            payload_len = websocket_content_len(&frame_header[0..2])?;
            masking_key.copy_from_slice(&frame_header[2..6]);
        }
        127 => {
            frame_header = vec![0; 12];
            buf_reader.read_exact(&mut frame_header)?;
            payload_len = websocket_content_len(&frame_header[0..8])?;
            masking_key.copy_from_slice(&frame_header[8..12]);
        }
        128..=u64::MAX => {
            panic!("The 7 bits which encode the logic for the payload length cannot be greater than 127")
        }
    }

    trace!("full payload len: {}", payload_len);
    trace!("masking key: {:02X?}", masking_key);

    let mut rec_msg = vec![0; payload_len as usize];
    buf_reader.read_exact(&mut rec_msg)?;

    let decoded: Vec<u8> = rec_msg
        .iter()
        .enumerate()
        .map(|(index, el)| el ^ masking_key[index % 4])
        .collect();

    let msg = Message::new(opcode, decoded);

    return Ok(msg);
}

fn websocket_content_len(data: &[u8]) -> Result<u64, Error> {
    let num_shifts = data.len();
    match num_shifts {
        0..=8 => Ok(data
            .iter()
            .rev()
            .enumerate()
            .map(|(idx, el)| (*el as u64) << 8 * (idx))
            .fold(0, |acc, x| acc | x)),
        _ => Err(Error::from(ErrorKind::InvalidData)),
    }
}

pub fn websocket_handshake(req: &HttpRequest) -> Result<String, Error> {
    let headers = match req {
        HttpRequest::Get {
            status_line: _,
            headers,
        } => headers,
        HttpRequest::Post {
            status_line: _,
            headers: _,
            body: _,
        } => {
            return {
                error!("A post request was made to perform the websocket handshake, but this does not follow rfc6455.");
                Err(Error::from(ErrorKind::Other))
            }
        }
    };

    let status_line = "HTTP/1.1 101 Switching Protocols";

    let sender_key = if let Some(key) = headers.get("Sec-WebSocket-Key") {
        key
    } else {
        error!("While trying to find the client's sender key, we did not find Sec-WebSocket-Key in the Request's headers.");
        return Err(Error::from(ErrorKind::InvalidData));
    };

    let encoded_data = web_socket_accept(sender_key);

    let handshake = format!("{status_line}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {encoded_data}\r\n\r\n");
    log::trace!("Handshake:\n{}", handshake);

    return Ok(handshake);
}

#[cfg(test)]
mod tests {
    use crate::websockets::web_socket_accept;

    #[test]
    fn web_socket_accept_header() {
        let actual = web_socket_accept("dGhlIHNhbXBsZSBub25jZQ==");
        assert_eq!(actual, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }
}
