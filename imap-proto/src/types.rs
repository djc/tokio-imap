use std::borrow::Cow;
use std::ops::RangeInclusive;

fn to_owned_cow<'a, T: ?Sized + ToOwned>(c: Cow<'a, T>) -> Cow<'static, T> {
    Cow::Owned(c.into_owned())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Request<'a>(pub Cow<'a, [u8]>, pub Cow<'a, [u8]>);

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum AttrMacro {
    All,
    Fast,
    Full,
}

#[derive(Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Response<'a> {
    Capabilities(Vec<Capability<'a>>),
    Continue {
        code: Option<ResponseCode<'a>>,
        information: Option<Cow<'a, str>>,
    },
    Done {
        tag: RequestId,
        status: Status,
        code: Option<ResponseCode<'a>>,
        information: Option<Cow<'a, str>>,
    },
    Data {
        status: Status,
        code: Option<ResponseCode<'a>>,
        information: Option<Cow<'a, str>>,
    },
    Expunge(u32),
    Vanished {
        earlier: bool,
        uids: Vec<std::ops::RangeInclusive<u32>>,
    },
    Fetch(u32, Vec<AttributeValue<'a>>),
    MailboxData(MailboxDatum<'a>),
}

impl<'a> Response<'a> {
    pub fn from_bytes(buf: &'a [u8]) -> crate::ParseResult {
        crate::parser::parse_response(buf)
    }
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
#[non_exhaustive]
pub enum ResponseCode<'a> {
    Alert,
    BadCharset(Option<Vec<Cow<'a, str>>>),
    Capabilities(Vec<Capability<'a>>),
    HighestModSeq(u64), // RFC 4551, section 3.1.1
    Parse,
    PermanentFlags(Vec<Cow<'a, str>>),
    ReadOnly,
    ReadWrite,
    TryCreate,
    UidNext(u32),
    UidValidity(u32),
    Unseen(u32),
    AppendUid(u32, Vec<UidSetMember>),
    CopyUid(u32, Vec<UidSetMember>, Vec<UidSetMember>),
    UidNotSticky,
    MetadataLongEntries(u64), // RFC 5464, section 4.2.1
    MetadataMaxSize(u64),     // RFC 5464, section 4.3
    MetadataTooMany,          // RFC 5464, section 4.3
    MetadataNoPrivate,        // RFC 5464, section 4.3
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UidSetMember {
    UidRange(RangeInclusive<u32>),
    Uid(u32),
}
impl From<RangeInclusive<u32>> for UidSetMember {
    fn from(x: RangeInclusive<u32>) -> Self {
        UidSetMember::UidRange(x)
    }
}
impl From<u32> for UidSetMember {
    fn from(x: u32) -> Self {
        UidSetMember::Uid(x)
    }
}

impl<'a> ResponseCode<'a> {
    pub fn into_owned(self) -> ResponseCode<'static> {
        match self {
            ResponseCode::Alert => ResponseCode::Alert,
            ResponseCode::BadCharset(v) => {
                ResponseCode::BadCharset(v.map(|vs| vs.into_iter().map(to_owned_cow).collect()))
            }
            ResponseCode::Capabilities(v) => {
                ResponseCode::Capabilities(v.into_iter().map(Capability::into_owned).collect())
            }
            ResponseCode::HighestModSeq(v) => ResponseCode::HighestModSeq(v),
            ResponseCode::Parse => ResponseCode::Parse,
            ResponseCode::PermanentFlags(v) => {
                ResponseCode::PermanentFlags(v.into_iter().map(to_owned_cow).collect())
            }
            ResponseCode::ReadOnly => ResponseCode::ReadOnly,
            ResponseCode::ReadWrite => ResponseCode::ReadWrite,
            ResponseCode::TryCreate => ResponseCode::TryCreate,
            ResponseCode::UidNext(v) => ResponseCode::UidNext(v),
            ResponseCode::UidValidity(v) => ResponseCode::UidValidity(v),
            ResponseCode::Unseen(v) => ResponseCode::Unseen(v),
            ResponseCode::AppendUid(a, b) => ResponseCode::AppendUid(a, b),
            ResponseCode::CopyUid(a, b, c) => ResponseCode::CopyUid(a, b, c),
            ResponseCode::UidNotSticky => ResponseCode::UidNotSticky,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum StatusAttribute {
    HighestModSeq(u64), // RFC 4551
    Messages(u32),
    Recent(u32),
    UidNext(u32),
    UidValidity(u32),
    Unseen(u32),
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Metadata {
    pub entry: String,
    pub value: Option<String>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum MailboxDatum<'a> {
    Exists(u32),
    Flags(Vec<Cow<'a, str>>),
    List {
        flags: Vec<Cow<'a, str>>,
        delimiter: Option<Cow<'a, str>>,
        name: Cow<'a, str>,
    },
    Search(Vec<u32>),
    Status {
        mailbox: Cow<'a, str>,
        status: Vec<StatusAttribute>,
    },
    Recent(u32),
    MetadataSolicited {
        mailbox: Cow<'a, str>,
        values: Vec<Metadata>,
    },
    MetadataUnsolicited {
        mailbox: Cow<'a, str>,
        values: Vec<Cow<'a, str>>,
    },
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum Capability<'a> {
    Imap4rev1,
    Auth(Cow<'a, str>),
    Atom(Cow<'a, str>),
}

impl<'a> Capability<'a> {
    pub fn into_owned(self) -> Capability<'static> {
        match self {
            Capability::Imap4rev1 => Capability::Imap4rev1,
            Capability::Auth(v) => Capability::Auth(to_owned_cow(v)),
            Capability::Atom(v) => Capability::Atom(to_owned_cow(v)),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
#[non_exhaustive]
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

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AttributeValue<'a> {
    BodySection {
        section: Option<SectionPath>,
        index: Option<u32>,
        data: Option<Cow<'a, [u8]>>,
    },
    BodyStructure(BodyStructure<'a>),
    Envelope(Box<Envelope<'a>>),
    Flags(Vec<Cow<'a, str>>),
    InternalDate(Cow<'a, str>),
    ModSeq(u64), // RFC 4551, section 3.3.2
    Rfc822(Option<Cow<'a, [u8]>>),
    Rfc822Header(Option<Cow<'a, [u8]>>),
    Rfc822Size(u32),
    Rfc822Text(Option<Cow<'a, [u8]>>),
    Uid(u32),
}

impl<'a> AttributeValue<'a> {
    pub fn into_owned(self) -> AttributeValue<'static> {
        match self {
            AttributeValue::BodySection {
                section,
                index,
                data,
            } => AttributeValue::BodySection {
                section,
                index,
                data: data.map(to_owned_cow),
            },
            AttributeValue::BodyStructure(body) => AttributeValue::BodyStructure(body.into_owned()),
            AttributeValue::Envelope(e) => AttributeValue::Envelope(Box::new(e.into_owned())),
            AttributeValue::Flags(v) => {
                AttributeValue::Flags(v.into_iter().map(to_owned_cow).collect())
            }
            AttributeValue::InternalDate(v) => AttributeValue::InternalDate(to_owned_cow(v)),
            AttributeValue::ModSeq(v) => AttributeValue::ModSeq(v),
            AttributeValue::Rfc822(v) => AttributeValue::Rfc822(v.map(to_owned_cow)),
            AttributeValue::Rfc822Header(v) => AttributeValue::Rfc822Header(v.map(to_owned_cow)),
            AttributeValue::Rfc822Size(v) => AttributeValue::Rfc822Size(v),
            AttributeValue::Rfc822Text(v) => AttributeValue::Rfc822Text(v.map(to_owned_cow)),
            AttributeValue::Uid(v) => AttributeValue::Uid(v),
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Eq, PartialEq)]
pub enum BodyStructure<'a> {
    Basic {
        common: BodyContentCommon<'a>,
        other: BodyContentSinglePart<'a>,
        extension: Option<BodyExtension<'a>>,
    },
    Text {
        common: BodyContentCommon<'a>,
        other: BodyContentSinglePart<'a>,
        lines: u32,
        extension: Option<BodyExtension<'a>>,
    },
    Message {
        common: BodyContentCommon<'a>,
        other: BodyContentSinglePart<'a>,
        envelope: Envelope<'a>,
        body: Box<BodyStructure<'a>>,
        lines: u32,
        extension: Option<BodyExtension<'a>>,
    },
    Multipart {
        common: BodyContentCommon<'a>,
        bodies: Vec<BodyStructure<'a>>,
        extension: Option<BodyExtension<'a>>,
    },
}

impl<'a> BodyStructure<'a> {
    pub fn into_owned(self) -> BodyStructure<'static> {
        match self {
            BodyStructure::Basic {
                common,
                other,
                extension,
            } => BodyStructure::Basic {
                common: common.into_owned(),
                other: other.into_owned(),
                extension: extension.map(|v| v.into_owned()),
            },
            BodyStructure::Text {
                common,
                other,
                lines,
                extension,
            } => BodyStructure::Text {
                common: common.into_owned(),
                other: other.into_owned(),
                lines,
                extension: extension.map(|v| v.into_owned()),
            },
            BodyStructure::Message {
                common,
                other,
                envelope,
                body,
                lines,
                extension,
            } => BodyStructure::Message {
                common: common.into_owned(),
                other: other.into_owned(),
                envelope: envelope.into_owned(),
                body: Box::new(body.into_owned()),
                lines,
                extension: extension.map(|v| v.into_owned()),
            },
            BodyStructure::Multipart {
                common,
                bodies,
                extension,
            } => BodyStructure::Multipart {
                common: common.into_owned(),
                bodies: bodies.into_iter().map(|v| v.into_owned()).collect(),
                extension: extension.map(|v| v.into_owned()),
            },
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct BodyContentCommon<'a> {
    pub ty: ContentType<'a>,
    pub disposition: Option<ContentDisposition<'a>>,
    pub language: Option<Vec<Cow<'a, str>>>,
    pub location: Option<Cow<'a, str>>,
}

impl<'a> BodyContentCommon<'a> {
    pub fn into_owned(self) -> BodyContentCommon<'static> {
        BodyContentCommon {
            ty: self.ty.into_owned(),
            disposition: self.disposition.map(|v| v.into_owned()),
            language: self
                .language
                .map(|v| v.into_iter().map(to_owned_cow).collect()),
            location: self.location.map(to_owned_cow),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct BodyContentSinglePart<'a> {
    pub id: Option<Cow<'a, str>>,
    pub md5: Option<Cow<'a, str>>,
    pub description: Option<Cow<'a, str>>,
    pub transfer_encoding: ContentEncoding<'a>,
    pub octets: u32,
}

impl<'a> BodyContentSinglePart<'a> {
    pub fn into_owned(self) -> BodyContentSinglePart<'static> {
        BodyContentSinglePart {
            id: self.id.map(to_owned_cow),
            md5: self.md5.map(to_owned_cow),
            description: self.description.map(to_owned_cow),
            transfer_encoding: self.transfer_encoding.into_owned(),
            octets: self.octets,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ContentType<'a> {
    pub ty: Cow<'a, str>,
    pub subtype: Cow<'a, str>,
    pub params: BodyParams<'a>,
}

impl<'a> ContentType<'a> {
    pub fn into_owned(self) -> ContentType<'static> {
        ContentType {
            ty: to_owned_cow(self.ty),
            subtype: to_owned_cow(self.subtype),
            params: body_param_owned(self.params),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ContentDisposition<'a> {
    pub ty: Cow<'a, str>,
    pub params: BodyParams<'a>,
}

impl<'a> ContentDisposition<'a> {
    pub fn into_owned(self) -> ContentDisposition<'static> {
        ContentDisposition {
            ty: to_owned_cow(self.ty),
            params: body_param_owned(self.params),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ContentEncoding<'a> {
    SevenBit,
    EightBit,
    Binary,
    Base64,
    QuotedPrintable,
    Other(Cow<'a, str>),
}

impl<'a> ContentEncoding<'a> {
    pub fn into_owned(self) -> ContentEncoding<'static> {
        match self {
            ContentEncoding::SevenBit => ContentEncoding::SevenBit,
            ContentEncoding::EightBit => ContentEncoding::EightBit,
            ContentEncoding::Binary => ContentEncoding::Binary,
            ContentEncoding::Base64 => ContentEncoding::Base64,
            ContentEncoding::QuotedPrintable => ContentEncoding::QuotedPrintable,
            ContentEncoding::Other(v) => ContentEncoding::Other(to_owned_cow(v)),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum BodyExtension<'a> {
    Num(u32),
    Str(Option<Cow<'a, str>>),
    List(Vec<BodyExtension<'a>>),
}

impl<'a> BodyExtension<'a> {
    pub fn into_owned(self) -> BodyExtension<'static> {
        match self {
            BodyExtension::Num(v) => BodyExtension::Num(v),
            BodyExtension::Str(v) => BodyExtension::Str(v.map(to_owned_cow)),
            BodyExtension::List(v) => {
                BodyExtension::List(v.into_iter().map(|v| v.into_owned()).collect())
            }
        }
    }
}

pub type BodyParams<'a> = Option<Vec<(Cow<'a, str>, Cow<'a, str>)>>;

fn body_param_owned<'a>(v: BodyParams<'a>) -> BodyParams<'static> {
    v.map(|v| {
        v.into_iter()
            .map(|(k, v)| (to_owned_cow(k), to_owned_cow(v)))
            .collect()
    })
}

#[derive(Debug, Eq, PartialEq)]
pub struct Envelope<'a> {
    pub date: Option<Cow<'a, [u8]>>,
    pub subject: Option<Cow<'a, [u8]>>,
    pub from: Option<Vec<Address<'a>>>,
    pub sender: Option<Vec<Address<'a>>>,
    pub reply_to: Option<Vec<Address<'a>>>,
    pub to: Option<Vec<Address<'a>>>,
    pub cc: Option<Vec<Address<'a>>>,
    pub bcc: Option<Vec<Address<'a>>>,
    pub in_reply_to: Option<Cow<'a, [u8]>>,
    pub message_id: Option<Cow<'a, [u8]>>,
}

impl<'a> Envelope<'a> {
    pub fn into_owned(self) -> Envelope<'static> {
        Envelope {
            date: self.date.map(to_owned_cow),
            subject: self.subject.map(to_owned_cow),
            from: self
                .from
                .map(|v| v.into_iter().map(|v| v.into_owned()).collect()),
            sender: self
                .sender
                .map(|v| v.into_iter().map(|v| v.into_owned()).collect()),
            reply_to: self
                .reply_to
                .map(|v| v.into_iter().map(|v| v.into_owned()).collect()),
            to: self
                .to
                .map(|v| v.into_iter().map(|v| v.into_owned()).collect()),
            cc: self
                .cc
                .map(|v| v.into_iter().map(|v| v.into_owned()).collect()),
            bcc: self
                .bcc
                .map(|v| v.into_iter().map(|v| v.into_owned()).collect()),
            in_reply_to: self.in_reply_to.map(to_owned_cow),
            message_id: self.message_id.map(to_owned_cow),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Address<'a> {
    pub name: Option<Cow<'a, [u8]>>,
    pub adl: Option<Cow<'a, [u8]>>,
    pub mailbox: Option<Cow<'a, [u8]>>,
    pub host: Option<Cow<'a, [u8]>>,
}

impl<'a> Address<'a> {
    pub fn into_owned(self) -> Address<'static> {
        Address {
            name: self.name.map(to_owned_cow),
            adl: self.adl.map(to_owned_cow),
            mailbox: self.mailbox.map(to_owned_cow),
            host: self.host.map(to_owned_cow),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestId(pub String);

impl RequestId {
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum State {
    NotAuthenticated,
    Authenticated,
    Selected,
    Logout,
}

// Body Structure

pub struct BodyFields<'a> {
    pub param: BodyParams<'a>,
    pub id: Option<Cow<'a, str>>,
    pub description: Option<Cow<'a, str>>,
    pub transfer_encoding: ContentEncoding<'a>,
    pub octets: u32,
}

impl<'a> BodyFields<'a> {
    pub fn into_owned(self) -> BodyFields<'static> {
        BodyFields {
            param: body_param_owned(self.param),
            id: self.id.map(to_owned_cow),
            description: self.description.map(to_owned_cow),
            transfer_encoding: self.transfer_encoding.into_owned(),
            octets: self.octets,
        }
    }
}

pub struct BodyExt1Part<'a> {
    pub md5: Option<Cow<'a, str>>,
    pub disposition: Option<ContentDisposition<'a>>,
    pub language: Option<Vec<Cow<'a, str>>>,
    pub location: Option<Cow<'a, str>>,
    pub extension: Option<BodyExtension<'a>>,
}

impl<'a> BodyExt1Part<'a> {
    pub fn into_owned(self) -> BodyExt1Part<'static> {
        BodyExt1Part {
            md5: self.md5.map(to_owned_cow),
            disposition: self.disposition.map(|v| v.into_owned()),
            language: self
                .language
                .map(|v| v.into_iter().map(to_owned_cow).collect()),
            location: self.location.map(to_owned_cow),
            extension: self.extension.map(|v| v.into_owned()),
        }
    }
}

pub struct BodyExtMPart<'a> {
    pub param: BodyParams<'a>,
    pub disposition: Option<ContentDisposition<'a>>,
    pub language: Option<Vec<Cow<'a, str>>>,
    pub location: Option<Cow<'a, str>>,
    pub extension: Option<BodyExtension<'a>>,
}

impl<'a> BodyExtMPart<'a> {
    pub fn into_owned(self) -> BodyExtMPart<'static> {
        BodyExtMPart {
            param: body_param_owned(self.param),
            disposition: self.disposition.map(|v| v.into_owned()),
            language: self
                .language
                .map(|v| v.into_iter().map(to_owned_cow).collect()),
            location: self.location.map(to_owned_cow),
            extension: self.extension.map(|v| v.into_owned()),
        }
    }
}
