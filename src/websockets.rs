use std::{
    io::{self, BufReader, Error, ErrorKind, Read},
    net::TcpStream, usize,
};

use base64::{engine::general_purpose, Engine as _};
use crypto::{digest::Digest, sha1::Sha1};
use log::{error, trace};

use crate::HttpRequest;

#[derive(Debug, Copy, Clone)]
pub enum OpCode {
    Continuation,
    Text,
    Binary,
    Reserved(u8),
    Close,
    Ping,
    Pong,
}

#[derive(Debug, PartialEq, Eq)]
pub struct OpCodeFromError {
    val: u8
}
#[derive(Debug, PartialEq, Eq)]
pub struct OpCodeIntoError;

impl Into<u8> for OpCode {

    fn into(self) -> u8 {
        match self {
            OpCode::Continuation => 0x0,
            OpCode::Text => 0x1,
            OpCode::Binary => 0x2,
            OpCode::Reserved(x) => x,
            OpCode::Close => 0x8,
            OpCode::Ping => 0x9,
            OpCode::Pong => 0xA,
        }
    }    
}

impl TryFrom<u8> for OpCode {
    type Error = OpCodeFromError;
    
    fn try_from(value: u8) -> Result<Self, OpCodeFromError> {
        match value {
            0x0 => Ok(OpCode::Continuation),
            0x1 => Ok(OpCode::Text),
            0x2 => Ok(OpCode::Binary),
            0x8 => Ok(OpCode::Close),
            0x9 => Ok(OpCode::Ping),
            0xA => Ok(OpCode::Pong),
            0x3 ..=0x7 => Ok(OpCode::Reserved(value)),
            0xB ..=0xF => Ok(OpCode::Reserved(value)),
            _ => Err(OpCodeFromError {val: value})
        }
    }

    
}
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Message {
    pub opcode: OpCode,
    pub body: Vec<u8>,
    masking_key: [u8; 4],
    fin_bit: u8, // 0b1000_0000
    len: usize,
    raw_data: Vec<u8>,
}

impl Message {
    fn new(opcode: OpCode, body: Vec<u8>) -> Self {
        // this always has fin bit = 1
        // this has no masking key

        let mut raw_data: Vec<u8> = Vec::new();
        // finbit = true.
        // first byte.
        let op_code: u8 = opcode.into();
        let fin_bit: u8 = 0b1000_0000;
        raw_data.push(fin_bit | op_code);

        let len = body.len();
        match len {
            0..=125 => raw_data.push(len.try_into().unwrap()),
            126..=65535 => {
                let len = (len as u16).to_le_bytes();
                raw_data.push(126);
                raw_data.extend(len.iter());
            }
            65536..= 4294967295=> {
                let len = (len as u32).to_le_bytes();
                raw_data.push(127);
                raw_data.extend(len.iter());
            }
            x if x > 4294967295 => {
                todo!("make a proper error")
            }
            4294967296_usize.. => todo!()
        }
        raw_data.extend(body.iter());
        
        Self { opcode, body, masking_key: [0; 4], fin_bit, len, raw_data  }
    }

    pub fn new_from_str(body: &str) -> Self {
        return Self::new(OpCode::Text, body.as_bytes().to_vec())
    }

    pub fn ping_message() -> Self {
        return Self::new(OpCode::Ping, "ping".as_bytes().to_vec())
    }

    pub fn to_vec(self) ->  Vec<u8> {
        return self.into()
    }

}


impl Into<Vec<u8>> for Message {

    fn into(self) -> Vec<u8> {
        return self.raw_data;
    }    
}


#[derive(Debug, PartialEq, Eq)]
pub struct MessageDecodeError{
    msg: String
}


impl TryFrom<Vec<u8>> for Message {
    type Error = MessageDecodeError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        // see  RFC 6455: https://www.rfc-editor.org/rfc/rfc6455.html#section-5.3
        let frame_header = &value[0..2];
        let opcode_u8 = (frame_header[0] & 0b0000_1111) as u8;
        let opcode = match OpCode::try_from(opcode_u8){
            Ok(x) => x,
            Err(code) => {
                    return Err(MessageDecodeError{msg: format!("client send unexpected opcode {}",code.val) })
                }
        };

        let mut payload_len = (frame_header[1] & 0b0111_1111) as u64;
        let masking_key: &[u8];
        let rec_msg: &[u8];

        match payload_len {
            0..=125 => {
                masking_key = &value[2..6];
                rec_msg= &value[6..(payload_len as usize)];
            }
            126 => {
                // frame_header = vec![0; 6];
                // buf_reader.read_exact(&mut frame_header)?;
                // payload_len = websocket_content_len(&frame_header[0..2])?;
                payload_len =  websocket_content_len(&value[2..4]).unwrap();
                masking_key =  &value[4..8];
                rec_msg= &value[8..(payload_len as usize)];
            }
            127 => {
                // frame_header = vec![0; 12];
                // buf_reader.read_exact(&mut frame_header)?;
                // payload_len = websocket_content_len(&frame_header[0..8])?;
                payload_len =  websocket_content_len(&value[2..10]).unwrap();
                masking_key =  &value[10..14];
                rec_msg= &value[14..(payload_len as usize)];
                // masking_key.copy_from_slice(&frame_header[8..12]);
            }
            128..=u64::MAX => {
                panic!("The 7 bits which encode the logic for the payload length cannot be greater than 127")
            }
        }

        trace!("full payload len: {}", payload_len);
        trace!("masking key: {:02X?}", masking_key);

        let rec_msg = vec![0; payload_len as usize];

        let decoded: Vec<u8> = rec_msg
            .iter()
            .enumerate()
            .map(|(index, el)| el ^ masking_key[index % 4])
            .collect();

        let msg = Message::new(opcode, decoded);

        return Ok(msg);

    }
}


pub fn ping_message() -> Vec<u8> {
    let mut payload: Vec<u8> = Vec::new();
    let op_code: u8 = OpCode::Ping.into();
    let fin_bit: u8 = 0b1000_0000;
    payload.push(fin_bit | op_code);

    let msg = "ping";

    let len: u8 = msg.len() as u8;
    payload.push(len);

    payload.extend_from_slice(msg.as_bytes());

    for byte in &payload {
        print!("{:02X} ", *byte);
    }
    payload
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
    let op_code: u8 = OpCode::Text.into();
    let fin_bit: u8 = 0b1000_0000;
    payload.push(fin_bit | op_code);

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
    let opcode_u8 = (frame_header[0] & 0b0000_1111) as u8;
    let opcode = match OpCode::try_from(opcode_u8){
        Ok(x) => x,
        Err(code) => {
                    return Err(Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("client send unexpected opcode: {}", code.val),
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
    use crate::websockets::{ping_message, web_socket_accept, Message};

    #[test]
    fn web_socket_accept_header() {
        let actual = web_socket_accept("dGhlIHNhbXBsZSBub25jZQ==");
        assert_eq!(actual, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }

    #[test]
    fn web_socket_message() {
        let msg = Message::new_from_str("Hello, World!");
        let msg_btytes: Vec<u8> = msg.into();
        assert_eq!(msg_btytes, vec![129, 13, 72, 101, 108, 108, 111, 44, 32, 87, 111, 114, 108, 100, 33]);
    }
    

    #[test]
    fn web_socket_ping() {
        let msg = ping_message();
        assert_eq!(msg, vec![137, 4, 112, 105, 110, 103]);
    }
}
