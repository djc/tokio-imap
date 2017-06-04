use bytes::{BufMut, BytesMut};

use std::io;
use std::str;

use tokio_core::net::TcpStream;
use tokio_io::codec::{Decoder, Encoder, Framed};
use tokio_tls::TlsStream;

pub type ImapTransport = Framed<TlsStream<TcpStream>, ImapCodec>;

#[allow(dead_code)]
pub enum State {
    NotAuthenticated,
    Authenticated,
    Selected,
    Logout,
}

pub struct ImapCodec;

enum Status {
    Ok,
    No,
    Bad,
    PreAuth,
    Bye,
}

#[derive(Debug)]
pub enum Command {
    Login(String, String),
}

#[derive(Debug)]
pub struct Request(pub String, pub Command);

impl ToString for Command {
    fn to_string(&self) -> String {
        match *self {
            Command::Login(ref user_name, ref password) => {
                format!("LOGIN {} {}", user_name, password)
            },
        }
    }
}

#[derive(Debug)]
pub enum Response {
    Status(Option<String>, String),
}

pub fn tag(n: u64) -> String {
    format!("A{:04}", n % 10000)
}

impl Decoder for ImapCodec {
    type Item = Response;
    type Error = io::Error;
    fn decode(&mut self, buf: &mut BytesMut)
             -> Result<Option<Response>, io::Error> {
        if let Some(n) = buf.iter().position(|b| *b == b'\n') {
            let mut line = buf.split_to(n - 1);
            let request_id = if line[0] == b'*' {
                let _ = line.split_to(2);
                None
            } else {
                let pos = line.iter().position(|b| *b == b' ').unwrap();
                let tag = line.split_to(pos);
                let _ = line.split_to(1);
                Some(str::from_utf8(&tag).unwrap().to_string())
            };
            let rest = str::from_utf8(line.get(..).unwrap()).unwrap().to_string();
            buf.split_to(2);
            Ok(Some(Response::Status(request_id, rest)))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for ImapCodec {
    type Item = Request;
    type Error = io::Error;
    fn encode(&mut self, msg: Self::Item, dst: &mut BytesMut)
             -> Result<(), io::Error> {
        dst.put(msg.0.as_bytes());
        dst.put(b' ');
        dst.put(msg.1.to_string().as_bytes());
        dst.put("\r\n");
        Ok(())
    }
}

