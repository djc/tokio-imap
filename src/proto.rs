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
        let res = match parser::parse(buf) {
            None => { return Ok(None); },
            Some((response, rsp_len)) => {
                // This SHOULD be acceptable/safe: BytesMut storage memory is
                // allocated on the heap and should not move. It will not be
                // freed as long as we keep a reference alive, which we do
                // by retaining a reference to the split buffer, below.
                let response = unsafe { mem::transmute(response) };
                Some((response, rsp_len))
            },
        };
        let (response, rsp_len) = res.unwrap();
        let raw = buf.split_to(rsp_len);
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

pub struct CommandBuilder { }

impl CommandBuilder {
    pub fn check() -> Command {
        let mut args = vec![];
        args.extend("CHECK".as_bytes());
        Command {
            args: args,
            next_state: None,
        }
    }

    pub fn fetch() -> FetchCommandEmpty {
        let mut args = vec![];
        args.extend("FETCH ".as_bytes());
        FetchCommandEmpty { args: args }
    }

    pub fn login(user_name: &str, password: &str) -> Command {
        let mut args = vec![];
        args.extend("LOGIN ".as_bytes());
        args.extend(user_name.as_bytes());
        args.extend(" ".as_bytes());
        args.extend(password.as_bytes());
        Command {
            args: args,
            next_state: Some(State::Authenticated),
        }
    }

    pub fn select(mailbox: &str) -> Command {
        let mut args = vec![];
        args.extend("SELECT ".as_bytes());
        args.extend(mailbox.as_bytes());
        Command {
            args: args,
            next_state: Some(State::Selected),
        }
    }
}

pub struct Command {
    args: Vec<u8>,
    next_state: Option<State>,
}

impl Command {
    pub fn to_parts(self) -> (Vec<u8>, Option<State>) {
        let Command { args, next_state } = self;
        (args, next_state)
    }
}

pub struct FetchCommandEmpty {
    args: Vec<u8>,
}

impl FetchBuilderMessages for FetchCommandEmpty {
    fn prepare(self) -> FetchCommandMessages {
        let FetchCommandEmpty { args } = self;
        FetchCommandMessages { args }
    }
}

pub struct FetchCommandMessages {
    args: Vec<u8>,
}

impl FetchBuilderMessages for FetchCommandMessages {
    fn prepare(self) -> FetchCommandMessages {
        let FetchCommandMessages { mut args } = self;
        args.push(b',');
        FetchCommandMessages { args }
    }
}

impl FetchCommandMessages {
    pub fn attr_macro(self, named: AttrMacro) -> FetchCommand {
        let FetchCommandMessages { mut args } = self;
        args.push(b' ');
        match named {
            AttrMacro::All => { args.extend("ALL".as_bytes()); },
            AttrMacro::Fast => { args.extend("FAST".as_bytes()); },
            AttrMacro::Full => { args.extend("FULL".as_bytes()); },
        }
        FetchCommand { args }
    }
}

pub trait FetchBuilderMessages where Self: Sized {
    fn prepare(self) -> FetchCommandMessages;

    fn uid(self, uid: u32) -> FetchCommandMessages {
        let FetchCommandMessages { mut args } = self.prepare();
        args.extend(uid.to_string().as_bytes());
        FetchCommandMessages { args }
    }

    fn range(self, start: u32, stop: u32) -> FetchCommandMessages {
        let FetchCommandMessages { mut args } = self.prepare();
        args.extend(start.to_string().as_bytes());
        args.push(b':');
        args.extend(stop.to_string().as_bytes());
        FetchCommandMessages { args }
    }

    fn all_after(self, start: u32) -> FetchCommandMessages {
        let FetchCommandMessages { mut args } = self.prepare();
        args.extend(start.to_string().as_bytes());
        args.extend(":*".as_bytes());
        FetchCommandMessages { args }
    }
}

pub struct FetchCommandAttributes {
    args: Vec<u8>,
}

impl FetchBuilderAttributes for FetchCommandMessages {
    fn prepare(self) -> FetchCommandAttributes {
        let FetchCommandMessages { mut args } = self;
        args.extend(" (".as_bytes());
        FetchCommandAttributes { args }
    }
}

impl FetchBuilderAttributes for FetchCommandAttributes {
    fn prepare(self) -> FetchCommandAttributes {
        let FetchCommandAttributes { mut args } = self;
        args.push(b' ');
        FetchCommandAttributes { args }
    }
}

pub trait FetchBuilderAttributes where Self: Sized {
    fn prepare(self) -> FetchCommandAttributes;
    fn attr(self, attr: Attribute) -> FetchCommandAttributes {
        let FetchCommandAttributes { mut args } = self.prepare();
        args.extend(match attr {
            Attribute::Envelope => "ENVELOPE",
            Attribute::Flags => "FLAGS",
            Attribute::InternalDate => "INTERNALDATE",
            Attribute::ModSeq => "MODSEQ",
            Attribute::Rfc822Size => "RFC822.SIZE",
        }.as_bytes());
        FetchCommandAttributes { args }
    }
}

pub struct FetchCommand {
    args: Vec<u8>,
}

pub trait FetchBuilderModifiers where Self: Sized {
    fn prepare(self) -> FetchCommand;
    fn build(self) -> Command {
        let FetchCommand { args } = self.prepare();
        Command { args, next_state: None }
    }
    fn changed_since(self, seq: u64) -> FetchCommand {
        let FetchCommand { mut args } = self.prepare();
        args.extend(" (CHANGEDSINCE ".as_bytes());
        args.extend(seq.to_string().as_bytes());
        args.push(b')');
        FetchCommand { args }
    }
}

impl FetchBuilderModifiers for FetchCommandAttributes {
    fn prepare(self) -> FetchCommand {
        let FetchCommandAttributes { mut args, .. } = self;
        args.push(b')');
        FetchCommand { args }
    }
}

impl FetchBuilderModifiers for FetchCommand {
    fn prepare(self) -> FetchCommand { self }
}

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
    Envelope,
    Flags,
    InternalDate,
    ModSeq, // RFC 4551, section 3.3.2
    Rfc822Size,
}

#[derive(Debug)]
pub enum AttributeValue<'a> {
    Envelope(Envelope<'a>),
    Flags(Vec<&'a str>),
    InternalDate(&'a str),
    ModSeq(u64), // RFC 4551, section 3.3.2
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
