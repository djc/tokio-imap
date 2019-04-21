#[derive(Debug, Eq, PartialEq)]
pub struct Request(pub RequestId, pub Vec<u8>);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AttrMacro {
    All,
    Fast,
    Full,
}

impl Copy for AttrMacro {}

#[derive(Debug, Eq, PartialEq)]
pub enum Response<'a> {
    Capabilities(Vec<&'a str>),
    Continue {
        code: Option<ResponseCode<'a>>,
        information: Option<&'a str>,
    },
    Done {
        tag: RequestId,
        status: Status,
        code: Option<ResponseCode<'a>>,
        information: Option<&'a str>,
    },
    Data {
        status: Status,
        code: Option<ResponseCode<'a>>,
        information: Option<&'a str>,
    },
    Expunge(u32),
    Fetch(u32, Vec<AttributeValue<'a>>),
    MailboxData(MailboxDatum<'a>),
    IDs(Vec<u32>),
}

#[derive(Debug, Eq, PartialEq)]
pub enum Status {
    Ok,
    No,
    Bad,
    PreAuth,
    Bye,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ResponseCode<'a> {
    Alert,
    BadCharset(Option<Vec<&'a str>>),
    Capabilities(Vec<&'a str>),
    HighestModSeq(u64), // RFC 4551, section 3.1.1
    Parse,
    PermanentFlags(Vec<&'a str>),
    ReadOnly,
    ReadWrite,
    TryCreate,
    UidNext(u32),
    UidValidity(u32),
    Unseen(u32),
}

#[derive(Debug, Eq, PartialEq)]
pub enum StatusAttribute {
    HighestModSeq(u64), // RFC 4551
    Messages(u32),
    Recent(u32),
    UidNext(u32),
    UidValidity(u32),
    Unseen(u32),
}

#[derive(Debug, Eq, PartialEq)]
pub enum MailboxDatum<'a> {
    Exists(u32),
    Flags(Vec<&'a str>),
    List {
        flags: Vec<&'a str>,
        delimiter: Option<&'a str>,
        name: &'a str,
    },
    Status {
        mailbox: &'a str,
        status: Vec<StatusAttribute>,
    },
    Recent(u32),
}

#[derive(Debug, Eq, PartialEq)]
pub enum Attribute {
    Body,
    Envelope,
    Flags,
    InternalDate,
    ModSeq, // RFC 4551, section 3.3.2
    Rfc822,
    Rfc822Size,
    Rfc822Text,
    Uid,
}

#[derive(Debug, Eq, PartialEq)]
pub enum MessageSection {
    Header,
    Mime,
    Text,
}

#[derive(Debug, Eq, PartialEq)]
pub enum SectionPath {
    Full(MessageSection),
    Part(Vec<u32>, Option<MessageSection>),
}

#[derive(Debug, Eq, PartialEq)]
pub struct BodyParam<'a> {
    pub key: &'a str,
    pub val: &'a str,
}

#[derive(Debug, Eq, PartialEq)]
pub struct BodyDisposition<'a> {
    pub disposition_type: &'a str,
    pub params: Option<Vec<(BodyParam<'a>)>>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum BodyExtension<'a> {
    Num(u32),
    Str(Option<&'a str>),
    List(Vec<BodyExtension<'a>>),
}

#[derive(Debug, Eq, PartialEq)]
pub struct BodyStructureText<'a> {
    pub media_subtype: &'a str,
    pub params: Option<Vec<BodyParam<'a>>>,
    pub id: Option<&'a str>,
    pub description: Option<&'a str>,
    pub encoding: &'a str,
    pub octets: u32,
    pub md5: Option<&'a str>,
    pub disposition: Option<BodyDisposition<'a>>,
    pub lang: Option<Vec<&'a str>>,
    pub loc: Option<&'a str>,
    pub lines: u32,
    pub extensions: Option<BodyExtension<'a>>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct BodyStructureBasic<'a> {
    pub media_type: &'a str,
    pub media_subtype: &'a str,
    pub params: Option<Vec<BodyParam<'a>>>,
    pub id: Option<&'a str>,
    pub description: Option<&'a str>,
    pub encoding: &'a str,
    pub octets: u32,
    pub md5: Option<&'a str>,
    pub disposition: Option<BodyDisposition<'a>>,
    pub lang: Option<Vec<&'a str>>,
    pub loc: Option<&'a str>,
    pub extensions: Option<BodyExtension<'a>>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct BodyStructureMessage<'a> {
    pub params: Option<Vec<BodyParam<'a>>>,
    pub id: Option<&'a str>,
    pub description: Option<&'a str>,
    pub encoding: &'a str,
    pub octets: u32,
    pub envelope: Box<Envelope<'a>>,
    pub body: Box<BodyStructure<'a>>,
    pub lines: u32,
}

#[derive(Debug, Eq, PartialEq)]
pub struct BodyStructureMultipart<'a> {
    pub bodies: Vec<BodyStructure<'a>>,
    pub media_subtype: &'a str,
    pub params: Option<Vec<BodyParam<'a>>>,
    pub disposition: Option<BodyDisposition<'a>>,
    pub lang: Option<Vec<&'a str>>,
    pub loc: Option<&'a str>,
    pub extensions: Option<BodyExtension<'a>>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum BodyStructure<'a> {
    Basic(BodyStructureBasic<'a>),
    Text(BodyStructureText<'a>),
    Message(BodyStructureMessage<'a>),
    Multipart(BodyStructureMultipart<'a>),
}

#[derive(Debug, Eq, PartialEq)]
pub enum AttributeValue<'a> {
    BodySection {
        section: Option<SectionPath>,
        index: Option<u32>,
        data: Option<&'a [u8]>,
    },
    BodyStructure(Box<BodyStructure<'a>>),
    Envelope(Box<Envelope<'a>>),
    Flags(Vec<&'a str>),
    InternalDate(&'a str),
    ModSeq(u64), // RFC 4551, section 3.3.2
    Rfc822(Option<&'a [u8]>),
    Rfc822Header(Option<&'a [u8]>),
    Rfc822Size(u32),
    Rfc822Text(Option<&'a [u8]>),
    Uid(u32),
}

#[derive(Debug, Eq, PartialEq)]
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

#[derive(Debug, Eq, PartialEq)]
pub struct Address<'a> {
    pub name: Option<&'a str>,
    pub adl: Option<&'a str>,
    pub mailbox: Option<&'a str>,
    pub host: Option<&'a str>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestId(pub String);

impl RequestId {
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum State {
    NotAuthenticated,
    Authenticated,
    Selected,
    Logout,
}
