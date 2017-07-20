use bytes::{BufMut, BytesMut};

use nom::{IResult, Needed};

use std::io;
use std::mem;
use std::str;

use tokio_core::net::TcpStream;
use tokio_io::codec::{Decoder, Encoder, Framed};
use tokio_tls::TlsStream;

use parser;

pub type ImapTransport = Framed<TlsStream<TcpStream>, ImapCodec>;

pub struct ImapCodec {
    decode_need_message_bytes: usize,
}

impl Default for ImapCodec {
    fn default() -> ImapCodec {
        ImapCodec { decode_need_message_bytes: 0 }
    }
}

impl<'a> Decoder for ImapCodec {
    type Item = ResponseData;
    type Error = io::Error;
    fn decode(&mut self, buf: &mut BytesMut)
             -> Result<Option<Self::Item>, io::Error> {
        if self.decode_need_message_bytes > buf.len() {
            return Ok(None);
        }
        let res = match parser::parse_response(buf) {
            IResult::Done(remaining, response) => {
                // This SHOULD be acceptable/safe: BytesMut storage memory is
                // allocated on the heap and should not move. It will not be
                // freed as long as we keep a reference alive, which we do
                // by retaining a reference to the split buffer, below.
                let response = unsafe { mem::transmute(response) };
                Some((response, buf.len() - remaining.len()))
            },
            IResult::Incomplete(Needed::Size(min)) => {
                self.decode_need_message_bytes = min;
                return Ok(None);
            },
            IResult::Incomplete(_) => {
                return Ok(None);
            },
            IResult::Error(err) => {
                panic!("error {} during parsing of {:?}", err, buf);
            },
        };
        let (response, rsp_len) = res.unwrap();
        let raw = buf.split_to(rsp_len);
        self.decode_need_message_bytes = 0;
        Ok(Some(ResponseData { raw, response }))
    }
}

impl Encoder for ImapCodec {
    type Item = Request;
    type Error = io::Error;
    fn encode(&mut self, msg: Self::Item, dst: &mut BytesMut)
             -> Result<(), io::Error> {
        dst.put(msg.0.as_bytes());
        dst.put(b' ');
        dst.put(&msg.1);
        dst.put("\r\n");
        Ok(())
    }
}

#[derive(Debug)]
pub struct Request(pub RequestId, pub Vec<u8>);

#[derive(Debug)]
pub enum AttrMacro {
    All,
    Fast,
    Full,
}

#[derive(Debug)]
pub struct ResponseData {
    raw: BytesMut,
    // This reference is really scoped to the lifetime of the `raw`
    // member, but unfortunately Rust does not allow that yet. It
    // is transmuted to `'static` by the `Decoder`, instead, and
    // references returned to callers of `ResponseData` are limited
    // to the lifetime of the `ResponseData` struct.
    pub response: Response<'static>,
}

impl ResponseData {
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
    Fetch(u32, Vec<AttributeValue<'a>>),
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
pub enum Attribute {
    Body,
    Envelope,
    Flags,
    InternalDate,
    ModSeq, // RFC 4551, section 3.3.2
    Rfc822,
    Rfc822Size,
    Uid,
}

#[derive(Debug)]
pub enum AttributeValue<'a> {
    Envelope(Envelope<'a>),
    Flags(Vec<&'a str>),
    InternalDate(&'a str),
    ModSeq(u64), // RFC 4551, section 3.3.2
    Rfc822(Option<&'a str>),
    Rfc822Size(u32),
    Uid(u32),
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

#[allow(dead_code)]
pub enum State {
    NotAuthenticated,
    Authenticated,
    Selected,
    Logout,
}
