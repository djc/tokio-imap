use nom::{self, IResult};
use std::str;
use proto::{Address, Attribute, Envelope, MailboxDatum};
use proto::{RequestId, Response, ResponseCode, Status};

fn crlf(c: u8) -> bool {
    c == b'\r' || c == b'\n'
}

fn list_wildcards(c: u8) -> bool {
    c == b'%' || c == b'*'
}

fn quoted_specials(c: u8) -> bool {
    c == b'"' || c == b'\\'
}

fn resp_specials(c: u8) -> bool {
    c == b']'
}

fn atom_specials(c: u8) -> bool {
    c == b'(' || c == b')' || c == b'{' || c == b' ' || c < 32 ||
    list_wildcards(c) || quoted_specials(c) || resp_specials(c)
}

fn atom_char(c: u8) -> bool {
    !atom_specials(c)
}

fn astring_char(c: u8) -> bool {
    atom_char(c) || resp_specials(c)
}

fn tag_char(c: u8) -> bool {
    c != b'+' && astring_char(c)
}

named!(quoted_data<&[u8], &str, u32>, map!(
    //escaped!(take_till_s!(quoted_specials), '\\', one_of!("\"\\")),
    take_till_s!(quoted_specials),
    |s| str::from_utf8(s).unwrap()
));

named!(quoted<&str>, do_parse!(
    tag_s!("\"") >>
    data: quoted_data >>
    tag_s!("\"") >>
    (data)
));

named!(status_ok<Status>, map!(tag_no_case!("OK"),
    |s| Status::Ok
));
named!(status_no<Status>, map!(tag_no_case!("NO"),
    |s| Status::No
));
named!(status_bad<Status>, map!(tag_no_case!("BAD"),
    |s| Status::Bad
));
named!(status_preauth<Status>, map!(tag_no_case!("PREAUTH"),
    |s| Status::PreAuth
));
named!(status_bye<Status>, map!(tag_no_case!("BYE"),
    |s| Status::Bye
));

named!(status<Status>, alt!(
    status_ok |
    status_no |
    status_bad |
    status_preauth |
    status_bye
));

named!(number<usize>, map!(nom::digit,
    |s| str::parse(str::from_utf8(s).unwrap()).unwrap()
));

named!(text<&str>, map!(take_till_s!(crlf),
    |s| str::from_utf8(s).unwrap()
));

named!(atom<&str>, map!(take_while1_s!(atom_char),
    |s| str::from_utf8(s).unwrap()
));

fn flag_extension(i: &[u8]) -> IResult<&[u8], &str> {
    if i.len() < 1 || i[0] != b'\\' {
        return IResult::Error(nom::ErrorKind::Custom(0));
    }
    let mut last = 0;
    for (idx, c) in i[1..].iter().enumerate() {
        last = idx;
        if !atom_char(*c) {
            break;
        }
    }
    IResult::Done(&i[last + 1..], str::from_utf8(&i[..last + 1]).unwrap())
}

named!(flag<&str>, alt!(flag_extension | atom));

named!(flag_list<Vec<&str>>, do_parse!(
    tag_s!("(") >>
    elements: opt!(do_parse!(
        flag0: flag >>
        flags: many0!(do_parse!(
            tag_s!(" ") >>
            flag: flag >>
            (flag)
        )) >> ({
            let mut res = vec![flag0];
            res.extend(flags);
            res
        })
    )) >>
    tag_s!(")") >> ({
       if elements.is_some() {
           elements.unwrap()
       } else {
           Vec::new()
       }
    })
));

named!(flag_perm<&str>, alt!(
    map!(tag_s!("\\*"), |s| str::from_utf8(s).unwrap()) |
    flag
));

named!(resp_text_code_permanent_flags<ResponseCode>, do_parse!(
    tag_s!("PERMANENTFLAGS (") >>
    elements: dbg_dmp!(opt!(do_parse!(
        flag0: flag_perm >>
        flags: many0!(do_parse!(
            tag_s!(" ") >>
            flag: flag_perm >>
            (flag)
        )) >> ({
            let mut res = vec![flag0];
            res.extend(flags);
            res
        })
    ))) >>
    tag_s!(")") >> ({
        ResponseCode::PermanentFlags(if elements.is_some() {
            elements.unwrap()
        } else {
            Vec::new()
        })
    })
));

named!(resp_text_code_read_only<ResponseCode>, do_parse!(
    tag_s!("READ-ONLY") >>
    (ResponseCode::ReadOnly)
));

named!(resp_text_code_read_write<ResponseCode>, do_parse!(
    tag_s!("READ-WRITE") >>
    (ResponseCode::ReadWrite)
));

named!(resp_text_code_try_create<ResponseCode>, do_parse!(
    tag_s!("TRYCREATE") >>
    (ResponseCode::TryCreate)
));

named!(resp_text_code_uid_validity<ResponseCode>, do_parse!(
    tag_s!("UIDVALIDITY ") >>
    num: number >>
    (ResponseCode::UidValidity(num))
));

named!(resp_text_code_uid_next<ResponseCode>, do_parse!(
    tag_s!("UIDNEXT ") >>
    num: number >>
    (ResponseCode::UidNext(num))
));

named!(resp_text_code<ResponseCode>, do_parse!(
    tag_s!("[") >>
    coded: alt!(
        resp_text_code_permanent_flags |
        resp_text_code_uid_validity |
        resp_text_code_uid_next |
        resp_text_code_read_only |
        resp_text_code_read_write |
        resp_text_code_try_create
    ) >>
    tag_s!("] ") >>
    (coded)
));

named!(capability<&str>, do_parse!(
    tag_s!(" ") >>
    atom: take_till1_s!(atom_specials) >>
    (str::from_utf8(atom).unwrap())
));

named!(capability_data<Response>, do_parse!(
    tag_s!("CAPABILITY") >>
    capabilities: many1!(capability) >>
    (Response::Capabilities(capabilities))
));

named!(mailbox_data_flags<Response>, do_parse!(
    tag_s!("FLAGS ") >>
    flags: flag_list >>
    (Response::MailboxData(MailboxDatum::Flags(flags)))
));

named!(mailbox_data_exists<Response>, do_parse!(
    num: number >>
    tag_s!(" EXISTS") >>
    (Response::MailboxData(MailboxDatum::Exists(num)))
));

named!(mailbox_data_recent<Response>, do_parse!(
    num: number >>
    tag_s!(" RECENT") >>
    (Response::MailboxData(MailboxDatum::Recent(num)))
));

named!(mailbox_data<Response>, alt!(
    mailbox_data_flags |
    mailbox_data_exists |
    mailbox_data_recent
));

named!(nstring<Option<&str>>, map!(
    alt!(
        map!(tag_s!("NIL"), |s| str::from_utf8(s).unwrap()) |
        quoted
    ),
    |s| if s == "NIL" { None } else { Some(s) }
));

named!(address<Address>, do_parse!(
    tag_s!("(") >>
    name: nstring >>
    tag_s!(" ") >>
    adl: nstring >>
    tag_s!(" ") >>
    mailbox: nstring >>
    tag_s!(" ") >>
    host: nstring >>
    tag_s!(")") >>
    (Address { name, adl, mailbox, host })
));

named!(opt_addresses<Option<Vec<Address>>>, alt!(
    map!(tag_s!("NIL"), |s| None) |
    do_parse!(
        tag_s!("(") >>
        addrs: many1!(address) >>
        tag_s!(")") >>
        (Some(addrs))
    )
));

named!(msg_att_envelope<Attribute>, do_parse!(
    tag_s!("ENVELOPE (") >>
    date: nstring >>
    tag_s!(" ") >>
    subject: nstring >>
    tag_s!(" ") >>
    from: opt_addresses >>
    tag_s!(" ") >>
    sender: opt_addresses >>
    tag_s!(" ") >>
    reply_to: opt_addresses >>
    tag_s!(" ") >>
    to: opt_addresses >>
    tag_s!(" ") >>
    cc: opt_addresses >>
    tag_s!(" ") >>
    bcc: opt_addresses >>
    tag_s!(" ") >>
    in_reply_to: nstring >>
    tag_s!(" ") >>
    message_id: nstring >>
    tag_s!(")") >> ({
        Attribute::Envelope(Envelope {
            date, subject, from, sender, reply_to, to, cc, bcc, in_reply_to, message_id
        })
    })
));

named!(msg_att_internal_date<Attribute>, do_parse!(
    tag_s!("INTERNALDATE ") >>
    date: nstring >>
    (Attribute::InternalDate(date.unwrap()))
));

named!(msg_att_flags<Attribute>, do_parse!(
    tag_s!("FLAGS ") >>
    flags: flag_list >>
    (Attribute::Flags(flags))
));

named!(msg_att_rfc822_size<Attribute>, do_parse!(
    tag_s!("RFC822.SIZE ") >>
    num: number >>
    (Attribute::Rfc822Size(num))
));

named!(msg_att<Attribute>, alt!(
    msg_att_envelope |
    msg_att_internal_date |
    msg_att_flags |
    msg_att_rfc822_size
));

named!(msg_att_list<Vec<Attribute>>, do_parse!(
    tag_s!("(") >>
    elements: do_parse!(
        attr0: msg_att >>
        attrs: many0!(do_parse!(
            tag_s!(" ") >>
            attr: msg_att >>
            (attr)
        )) >> ({
            let mut res = vec![attr0];
            res.extend(attrs);
            res
        })
    ) >>
    tag_s!(")") >>
    (elements)
));

named!(message_data_fetch<Response>, do_parse!(
    num: number >>
    tag_s!(" FETCH ") >>
    attrs: msg_att_list >>
    (Response::Fetch(num, attrs))
));

named!(message_data_expunge<Response>, do_parse!(
    num: number >>
    tag_s!(" EXPUNGE") >>
    (Response::Expunge(num))
));

named!(tag<RequestId>, map!(take_while1_s!(tag_char),
    |s| RequestId(str::from_utf8(s).unwrap().to_string())
));

named!(response_tagged<Response>, do_parse!(
    tag: tag >>
    tag_s!(" ") >>
    status: status >>
    tag_s!(" ") >>
    code: opt!(resp_text_code) >>
    text: text >>
    (Response::Done(tag, status, code, text))
));

named!(response_done<Response>, alt!(response_tagged));

named!(resp_cond_untagged<Response>, do_parse!(
    status: status >>
    tag_s!(" ") >>
    code: opt!(resp_text_code) >>
    text: text >>
    (Response::Data(status, code, text))
));

named!(response_data<Response>, do_parse!(
    tag_s!("* ") >>
    contents: alt!(
        resp_cond_untagged |
        capability_data |
        mailbox_data |
        message_data_expunge |
        message_data_fetch
    ) >>
    (contents)
));

named!(response<Response>, alt!(
    response_data |
    response_done
));

pub fn parse(msg: &str) -> Response {
    match response(msg.as_bytes()) {
        IResult::Done(_, res) => res,
        IResult::Error(err) => panic!("problems parsing template source: {}", err),
        IResult::Incomplete(_) => panic!("parsing incomplete"),
    }
}
