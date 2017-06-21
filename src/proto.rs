use bytes::{BufMut, BytesMut};

use std::io;
use std::mem;
use std::str;

use tokio_core::net::TcpStream;
use tokio_io::codec::{Decoder, Encoder, Framed};
use tokio_tls::TlsStream;

use parser;

pub type ImapTransport = Framed<TlsStream<TcpStream>, ImapCodec>;

pub struct ImapCodec;

impl<'a> Decoder for ImapCodec {
    type Item = ResponseData;
    type Error = io::Error;
    fn decode(&mut self, buf: &mut BytesMut)
             -> Result<Option<Self::Item>, io::Error> {
        if let Some(n) = buf.iter().position(|b| *b == b'\n') {
            let msg = buf.split_to(n - 1);
            buf.split_to(2);
            let owned = str::from_utf8(&msg).unwrap().to_string();
            Ok(Some(ResponseData::new(owned)))
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

#[derive(Debug)]
pub struct Request(pub RequestId, pub Command);

#[derive(Debug)]
pub enum Command {
    Check,
    Fetch(SequenceSet, MessageData),
    Login(String, String),
    Select(String),
}

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
pub struct ResponseData {
    raw: String,
    pub response: Response<'static>,
}

impl ResponseData {
    fn new<'a>(raw: String) -> ResponseData {
        // This SHOULD be acceptable/safe: the String memory is allocated on
        // the heap, so that moving the String itself does not invalidate
        // references to the string data contained in the parsed Response.
        let rsp = unsafe { mem::transmute(parser::parse(&raw)) };
        ResponseData {
            raw: raw,
            response: rsp,
        }
    }
    pub fn request_id(&self) -> Option<&RequestId> {
        match self.response {
            Response::Done(ref req_id, ..) => Some(req_id),
            _ => None,
        }
    }
    pub fn parsed(&self) -> &Response {
        unsafe { mem::transmute(&self.response) }
    }
}

#[derive(Debug)]
pub enum Response<'a> {
    Capabilities(Vec<&'a str>),
    Done(RequestId, Status, Option<ResponseCode<'a>>, Option<&'a str>),
    Data(Status, Option<ResponseCode<'a>>, Option<&'a str>),
    Expunge(u32),
    Fetch(u32, Vec<Attribute<'a>>),
    MailboxData(MailboxDatum<'a>),
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum Status {
    Ok,
    No,
    Bad,
    PreAuth,
    Bye,
}

#[derive(Debug)]
pub enum ResponseCode<'a> {
    HighestModSeq(u64), // RFC 4551, section 3.1.1
    PermanentFlags(Vec<&'a str>),
    ReadOnly,
    ReadWrite,
    TryCreate,
    UidNext(u32),
    UidValidity(u32),
}

#[derive(Debug)]
pub enum MailboxDatum<'a> {
    Exists(u32),
    Flags(Vec<&'a str>),
    Recent(u32),
}

#[derive(Debug)]
pub enum Attribute<'a> {
    Envelope(Envelope<'a>),
    Flags(Vec<&'a str>),
    InternalDate(&'a str),
    Rfc822Size(u32),
}

#[derive(Debug)]
pub struct Envelope<'a> {
    pub date: Option<&'a str>,
    pub subject: Option<&'a str>,
    pub from: Option<Vec<Address<'a>>>,
    pub sender: Option<Vec<Address<'a>>>,
    pub reply_to: Option<Vec<Address<'a>>>,
    pub to: Option<Vec<Address<'a>>>,
    pub cc: Option<Vec<Address<'a>>>,
    pub bcc: Option<Vec<Address<'a>>>,
    pub in_reply_to: Option<&'a str>,
    pub message_id: Option<&'a str>,
}

#[derive(Debug)]
pub struct Address<'a> {
    pub name: Option<&'a str>,
    pub adl: Option<&'a str>,
    pub mailbox: Option<&'a str>,
    pub host: Option<&'a str>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestId(pub String);

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

#[allow(dead_code)]
pub enum State {
    NotAuthenticated,
    Authenticated,
    Selected,
    Logout,
}
