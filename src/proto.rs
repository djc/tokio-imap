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
pub enum MessageData {
    All,
    Fast,
    Full,
}

impl ToString for MessageData {
    fn to_string(&self) -> String {
        use self::MessageData::*;
        match *self {
            All => "ALL".to_string(),
            Fast => "FAST".to_string(),
            Full => "FULL".to_string(),
        }
    }
}

#[derive(Debug)]
pub enum SequenceSet {
    Range(usize, usize),
}

impl ToString for SequenceSet {
    fn to_string(&self) -> String {
        use self::SequenceSet::*;
        match *self {
            Range(start, stop) => format!("{}:{}", start, stop),
        }
    }
}

#[derive(Debug)]
pub enum Command {
    Check,
    Fetch(SequenceSet, MessageData),
    Login(String, String),
    Select(String),
}

#[derive(Debug)]
pub struct Request(pub RequestId, pub Command);

impl ToString for Command {
    fn to_string(&self) -> String {
        match *self {
            Command::Check => {
                format!("CHECK")
            },
            Command::Fetch(ref set, ref items) => {
                format!("FETCH {} {}", &set.to_string(), &items.to_string())
            },
            Command::Login(ref user_name, ref password) => {
                format!("LOGIN {} {}", user_name, password)
            },
            Command::Select(ref mailbox) => {
                format!("SELECT {}", mailbox)
            },
        }
    }
}

#[derive(Debug)]
pub enum Response {
    Status(Option<RequestId>, String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestId(String);

impl RequestId {
    fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

pub struct IdGenerator {
    next: u64,
}

impl IdGenerator {
    pub fn new() -> IdGenerator {
        IdGenerator { next: 0 }
    }
}

impl Iterator for IdGenerator {
    type Item = RequestId;
    fn next(&mut self) -> Option<Self::Item> {
        self.next += 1;
        Some(RequestId(format!("A{:04}", self.next % 10000)))
    }
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
                Some(RequestId(str::from_utf8(&tag).unwrap().to_string()))
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

