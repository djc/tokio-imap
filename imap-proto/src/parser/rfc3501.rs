//!
//! https://tools.ietf.org/html/rfc3501
//!
//! INTERNET MESSAGE ACCESS PROTOCOL
//!

// rustfmt doesn't do a very good job on nom parser invocations.
#![cfg_attr(rustfmt, rustfmt_skip)]
#![cfg_attr(feature = "cargo-clippy", allow(redundant_closure))]

use nom::IResult;

use std::str;

use crate::parser::rfc4551;
use types::*;
use core::*;
use body::*;


fn tag_char(c: u8) -> bool {
    c != b'+' && astring_char(c)
}

named!(status_ok<Status>, map!(tag_no_case!("OK"),
    |_s| Status::Ok
));
named!(status_no<Status>, map!(tag_no_case!("NO"),
    |_s| Status::No
));
named!(status_bad<Status>, map!(tag_no_case!("BAD"),
    |_s| Status::Bad
));
named!(status_preauth<Status>, map!(tag_no_case!("PREAUTH"),
    |_s| Status::PreAuth
));
named!(status_bye<Status>, map!(tag_no_case!("BYE"),
    |_s| Status::Bye
));

named!(status<Status>, alt!(
    status_ok |
    status_no |
    status_bad |
    status_preauth |
    status_bye
));

named!(mailbox<&str>, map!(
    map_res!(astring, str::from_utf8),
    |s| {
        if s.eq_ignore_ascii_case("INBOX") {
            "INBOX"
        } else {
            s
        }
    }
));

named!(flag_extension<&str>, map_res!(
    recognize!(pair!(tag!("\\"), take_while!(atom_char))),
    str::from_utf8
));

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
    map_res!(tag_s!("\\*"), str::from_utf8) |
    flag
));

named!(resp_text_code_alert<ResponseCode>, do_parse!(
    tag_s!("ALERT") >>
    (ResponseCode::Alert)
));

named!(resp_text_code_badcharset<ResponseCode>, do_parse!(
    tag_s!("BADCHARSET") >>
    ch: opt!(do_parse!(
        tag_s!(" (") >>
        charset0: map_res!(astring, str::from_utf8) >>
        charsets: many0!(do_parse!(
            tag_s!(" ") >>
            charset: map_res!(astring, str::from_utf8) >>
            (charset)
        )) >>
        tag_s!(")") >> ({
            let mut res = vec![charset0];
            res.extend(charsets);
            res
        })
    )) >>
    (ResponseCode::BadCharset(ch))
));

named!(capability_list<Vec<&str>>, do_parse!(
    capabilities1: many_till!(capability, tag_s!(" IMAP4rev1")) >>
    capabilities2: many0!(capability) >> ({
        let mut v = Vec::with_capacity(10);
        v.extend(capabilities1.0);
        v.push("IMAP4rev1");
        v.extend(capabilities2);
        v
    })
));

named!(resp_text_code_capability<ResponseCode>, do_parse!(
    tag_s!("CAPABILITY") >>
    capabilities: capability_list >>
    (ResponseCode::Capabilities(capabilities))
));

named!(resp_text_code_parse<ResponseCode>, do_parse!(
    tag_s!("PARSE") >>
    (ResponseCode::Parse)
));

named!(resp_text_code_permanent_flags<ResponseCode>, do_parse!(
    tag_s!("PERMANENTFLAGS (") >>
    elements: opt!(do_parse!(
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
    )) >>
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

named!(resp_text_code_unseen<ResponseCode>, do_parse!(
    tag_s!("UNSEEN ") >>
    num: number >>
    (ResponseCode::Unseen(num))
));

named!(resp_text_code<ResponseCode>, do_parse!(
    tag_s!("[") >>
    coded: alt!(
        resp_text_code_alert |
        resp_text_code_badcharset |
        resp_text_code_capability |
        resp_text_code_parse |
        resp_text_code_permanent_flags |
        resp_text_code_uid_validity |
        resp_text_code_uid_next |
        resp_text_code_unseen |
        resp_text_code_read_only |
        resp_text_code_read_write |
        resp_text_code_try_create |
        rfc4551::resp_text_code_highest_mod_seq
    ) >>
    // Per the spec, the closing tag should be "] ".
    // See `resp_text` for more on why this is done differently.
    tag_s!("]") >>
    (coded)
));

named!(capability<&str>, do_parse!(
    tag_s!(" ") >>
    atom: map_res!(take_till1_s!(atom_specials), str::from_utf8) >>
    (atom)
));

named!(capability_data<Response>, do_parse!(
    tag_s!("CAPABILITY") >>
    capabilities: capability_list >>
    (Response::Capabilities(capabilities))
));

named!(mailbox_data_search<Response>, do_parse!(
    tag_s!("SEARCH") >>
    ids: many0!(do_parse!(
        tag_s!(" ") >>
        id: number >>
        (id)
    )) >>
    (Response::IDs(ids))
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

named!(mailbox_list<(Vec<&str>, Option<&str>, &str)>, do_parse!(
    flags: flag_list >>
    tag_s!(" ") >>
    delimiter: alt!(
        map!(map_res!(quoted, str::from_utf8), |v| Some(v)) |
        map!(nil, |_| None)
    ) >>
    tag_s!(" ") >>
    name: mailbox >>
    ((flags, delimiter, name))
));

named!(mailbox_data_list<Response>, do_parse!(
    tag_s!("LIST ") >>
    data: mailbox_list >>
    (Response::MailboxData(MailboxDatum::List {
        flags: data.0,
        delimiter: data.1,
        name: data.2,
    }))
));

named!(mailbox_data_lsub<Response>, do_parse!(
    tag_s!("LSUB ") >>
    data: mailbox_list >>
    (Response::MailboxData(MailboxDatum::List {
        flags: data.0,
        delimiter: data.1,
        name: data.2,
    }))
));

// Unlike `status_att` in the RFC syntax, this includes the value,
// so that it can return a valid enum object instead of just a key.
named!(status_att<StatusAttribute>, alt!(
    rfc4551::status_att_val_highest_mod_seq |
    do_parse!(
        key: alt!(
            tag_s!("MESSAGES") |
            tag_s!("RECENT") |
            tag_s!("UIDNEXT") |
            tag_s!("UIDVALIDITY") |
            tag_s!("UNSEEN")
        ) >>
        tag_s!(" ") >>
        val: number >>
        (match key {
            b"MESSAGES" => StatusAttribute::Messages(val),
            b"RECENT" => StatusAttribute::Recent(val),
            b"UIDNEXT" => StatusAttribute::UidNext(val),
            b"UIDVALIDITY" => StatusAttribute::UidValidity(val),
            b"UNSEEN" => StatusAttribute::Unseen(val),
            _ => panic!("invalid status key {}", str::from_utf8(key).unwrap()),
        }))
));

named!(status_att_list<Vec<StatusAttribute>>, do_parse!(
    first: status_att >>
    rest: many0!(do_parse!(
        tag_s!(" ") >>
        status: status_att >>
        (status)
    )) >>
    ({
        let mut res = rest;
        res.insert(0, first);
        res
    })
));

named!(mailbox_data_status<Response>, do_parse!(
    tag_s!("STATUS ") >>
    mailbox: mailbox >>
    tag_s!(" (") >>
    status: status_att_list >>
    tag_s!(")") >>
    (Response::MailboxData(MailboxDatum::Status {
        mailbox,
        status,
    }))
));

named!(mailbox_data_recent<Response>, do_parse!(
    num: number >>
    tag_s!(" RECENT") >>
    (Response::MailboxData(MailboxDatum::Recent(num)))
));

named!(mailbox_data<Response>, alt!(
    mailbox_data_flags |
    mailbox_data_exists |
    mailbox_data_list |
    mailbox_data_lsub |
    mailbox_data_status |
    mailbox_data_recent |
    mailbox_data_search
));

// An address structure is a parenthesized list that describes an
// electronic mail address.
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
    (Address {
        name: name.map(|s| str::from_utf8(s).unwrap()),
        adl: adl.map(|s| str::from_utf8(s).unwrap()),
        mailbox: mailbox.map(|s| str::from_utf8(s).unwrap()),
        host: host.map(|s| str::from_utf8(s).unwrap()),
    })
));

named!(opt_addresses<Option<Vec<Address>>>, alt!(
    map!(nil, |_s| None) |
    do_parse!(
        tag_s!("(") >>
        addrs: separated_nonempty_list!(opt!(tag!(" ")), address) >>
        tag_s!(")") >>
        (Some(addrs))
    )
));

named!(msg_att_envelope<AttributeValue>, do_parse!(
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
        AttributeValue::Envelope(Box::new(Envelope {
            date: date.map(|s| str::from_utf8(s).unwrap()),
            subject: subject.map(|s| str::from_utf8(s).unwrap()),
            from,
            sender,
            reply_to,
            to,
            cc,
            bcc,
            in_reply_to: in_reply_to.map(|s| str::from_utf8(s).unwrap()),
            message_id: message_id.map(|s| str::from_utf8(s).unwrap()),
        }))
    })
));

named!(msg_att_internal_date<AttributeValue>, do_parse!(
    tag_s!("INTERNALDATE ") >>
    date: nstring >>
    (AttributeValue::InternalDate(str::from_utf8(date.unwrap()).unwrap()))
));

named!(msg_att_flags<AttributeValue>, do_parse!(
    tag_s!("FLAGS ") >>
    flags: flag_list >>
    (AttributeValue::Flags(flags))
));

named!(msg_att_rfc822<AttributeValue>, do_parse!(
    tag_s!("RFC822 ") >>
    raw: nstring >>
    (AttributeValue::Rfc822(raw))
));

named!(msg_att_rfc822_header<AttributeValue>, do_parse!(
    tag_s!("RFC822.HEADER ") >>
    opt!(tag_s!(" ")) >> // extra space workaround for DavMail
    raw: nstring >>
    (AttributeValue::Rfc822Header(raw))
));

named!(msg_att_rfc822_size<AttributeValue>, do_parse!(
    tag_s!("RFC822.SIZE ") >>
    num: number >>
    (AttributeValue::Rfc822Size(num))
));

named!(msg_att_rfc822_text<AttributeValue>, do_parse!(
    tag_s!("RFC822.TEXT ") >>
    raw: nstring >>
    (AttributeValue::Rfc822Text(raw))
));

named!(msg_att_uid<AttributeValue>, do_parse!(
    tag_s!("UID ") >>
    num: number >>
    (AttributeValue::Uid(num))
));

named!(msg_att<AttributeValue>, alt!(
    msg_att_body_section |
    msg_att_envelope |
    msg_att_internal_date |
    msg_att_flags |
    rfc4551::msg_att_mod_seq |
    msg_att_rfc822 |
    msg_att_rfc822_header |
    msg_att_rfc822_size |
    msg_att_rfc822_text |
    msg_att_uid
));

named!(msg_att_list<Vec<AttributeValue>>, do_parse!(
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

named!(tag<RequestId>, map!(
    map_res!(take_while1_s!(tag_char), str::from_utf8),
    |s| RequestId(s.to_string())
));

// This is not quite according to spec, which mandates the following:
//     ["[" resp-text-code "]" SP] text
// However, examples in RFC 4551 (Conditional STORE) counteract this by giving
// examples of `resp-text` that do not include the trailing space and text.
named!(resp_text<(Option<ResponseCode>, Option<&str>)>, do_parse!(
    code: opt!(resp_text_code) >>
    text: text >>
    ({
        let res = if text.is_empty() {
            None
        } else if code.is_some() {
            Some(&text[1..])
        } else {
            Some(text)
        };
        (code, res)
    })
));

named!(continue_req<Response>, do_parse!(
    tag_s!("+ ") >>
    text: resp_text >> // TODO: base64
    tag_s!("\r\n") >>
    (Response::Continue {
        code: text.0,
        information: text.1,
    })
));

named!(response_tagged<Response>, do_parse!(
    tag: tag >>
    tag_s!(" ") >>
    status: status >>
    tag_s!(" ") >>
    text: resp_text >>
    tag_s!("\r\n") >>
    (Response::Done {
        tag,
        status,
        code: text.0,
        information: text.1,
    })
));

named!(resp_cond<Response>, do_parse!(
    status: status >>
    tag_s!(" ") >>
    text: resp_text >>
    (Response::Data {
        status,
        code: text.0,
        information: text.1,
    })
));

named!(response_data<Response>, do_parse!(
    tag_s!("* ") >>
    contents: alt!(
        resp_cond |
        mailbox_data |
        message_data_expunge |
        message_data_fetch |
        capability_data
    ) >>
    tag_s!("\r\n") >>
    (contents)
));

named!(response<Response>, alt!(
    continue_req |
    response_data |
    response_tagged
));

pub type ParseResult<'a> = IResult<&'a [u8], Response<'a>>;

pub fn parse_response(msg: &[u8]) -> ParseResult {
    response(msg)
}


#[cfg(test)]
mod tests {
    use nom;
    use super::parse_response;
    use types::*;

    #[test]
    fn test_number_overflow() {
        match parse_response(b"* 2222222222222222222222222222222222222222222C\r\n") {
            Err(_) => {},
            _ => panic!("error required for integer overflow"),
        }
    }

    #[test]
    fn test_unseen() {
        match parse_response(b"* OK [UNSEEN 3] Message 3 is first unseen\r\n").unwrap() {
            (_, Response::Data {
                status: Status::Ok,
                code: Some(ResponseCode::Unseen(3)),
                information: Some("Message 3 is first unseen"),
            }) => {},
            rsp @ _ => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_body_text() {
        match parse_response(b"* 2 FETCH (BODY[TEXT] {3}\r\nfoo)\r\n") {
            Ok((_, Response::Fetch(_, attrs))) => {
                let body = &attrs[0];
                assert_eq!(body, &AttributeValue::BodySection {
                    section: Some(SectionPath::Full(MessageSection::Text)),
                    index: None,
                    data: Some(b"foo"),
                }, "body = {:?}", body);
            },
            rsp @ _ => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_status() {
        match parse_response(b"* STATUS blurdybloop (MESSAGES 231 UIDNEXT 44292)\r\n") {
            Ok((_, Response::MailboxData(MailboxDatum::Status { mailbox, status }))) => {
                assert_eq!(mailbox, "blurdybloop");
                assert_eq!(status, [
                    StatusAttribute::Messages(231),
                    StatusAttribute::UidNext(44292),
                ]);
            },
            rsp @ _ => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_notify() {
        match parse_response(b"* 3501 EXPUNGE\r\n") {
            Ok((_, Response::Expunge(3501))) => {},
            rsp @ _ => panic!("unexpected response {:?}", rsp),
        }
        match parse_response(b"* 3501 EXISTS\r\n") {
            Ok((_, Response::MailboxData(MailboxDatum::Exists(3501)))) => {},
            rsp @ _ => panic!("unexpected response {:?}", rsp),
        }
        match parse_response(b"+ idling\r\n") {
            Ok((_, Response::Continue { code: None, information: Some("idling") })) => {},
            rsp @ _ => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_search() {
        match parse_response(b"* SEARCH\r\n") {
            Ok((_, Response::IDs(ids))) => {
                assert!(ids.is_empty());
            },
            rsp @ _ => panic!("unexpected response {:?}", rsp),
        }
        match parse_response(b"* SEARCH 12345 67890\r\n") {
            Ok((_, Response::IDs(ids))) => {
                assert_eq!(ids[0], 12345);
                assert_eq!(ids[1], 67890);
            },
            rsp @ _ => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_uid_fetch() {
        match parse_response(b"* 4 FETCH (UID 71372 RFC822.HEADER {10275}\r\n") {
            Err(nom::Err::Incomplete(nom::Needed::Size(10275))) => {},
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_list() {
        match ::parser::rfc3501::mailbox(b"iNboX ") {
            Ok((_, mb)) => {
                assert_eq!(mb, "INBOX");
            },
            rsp => panic!("unexpected response {:?}", rsp),
        }

        match parse_response(b"* LIST (\\HasNoChildren) \".\" INBOX.Tests\r\n") {
            Ok((_, Response::MailboxData(_))) => {},
            rsp @ _ => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_uid_fetch_extra_space() {
        // DavMail inserts an extra space after RFC822.HEADER
        match parse_response(b"* 4 FETCH (UID 71372 RFC822.HEADER  {10275}\r\n") {
            Err(nom::Err::Incomplete(nom::Needed::Size(10275))) => {},
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_envelope() {
        let env = r#"ENVELOPE ("Wed, 17 Jul 1996 02:23:25 -0700 (PDT)" "IMAP4rev1 WG mtg summary and minutes" (("Terry Gray" NIL "gray" "cac.washington.edu")) (("Terry Gray" NIL "gray" "cac.washington.edu")) (("Terry Gray" NIL "gray" "cac.washington.edu")) ((NIL NIL "imap" "cac.washington.edu")) ((NIL NIL "minutes" "CNRI.Reston.VA.US") ("John Klensin" NIL "KLENSIN" "MIT.EDU")) NIL NIL "<B27397-0100000@cac.washington.edu>") "#;
        match ::parser::rfc3501::msg_att_envelope(env.as_bytes()) {
            Ok((_, AttributeValue::Envelope(_))) => {},
            rsp @ _ => panic!("unexpected response {:?}", rsp)
        }
    }

    #[test]
    fn test_opt_addresses() {
        let addr = b"((NIL NIL \"minutes\" \"CNRI.Reston.VA.US\") (\"John Klensin\" NIL \"KLENSIN\" \"MIT.EDU\")) ";
            match ::parser::rfc3501::opt_addresses(addr) {
            Ok((_, _addresses)) => {},
            rsp @ _ => panic!("unexpected response {:?}", rsp)
        }
    }

    #[test]
    fn test_addresses() {
        match ::parser::rfc3501::address(b"(\"John Klensin\" NIL \"KLENSIN\" \"MIT.EDU\") ") {
            Ok((_, _address)) => {},
            rsp @ _ => panic!("unexpected response {:?}", rsp)
        }
    }

    #[test]
    fn test_response_codes() {
        match parse_response(b"* OK [ALERT] Alert!\r\n") {
            Ok((_, Response::Data { status: Status::Ok, code: Some(ResponseCode::Alert), information: Some("Alert!") })) => {}
            rsp @ _ => panic!("unexpected response {:?}", rsp)
        }

        match parse_response(b"* NO [PARSE] Something\r\n") {
            Ok((_, Response::Data { status: Status::No, code: Some(ResponseCode::Parse), information: Some("Something") })) => {}
            rsp @ _ => panic!("unexpected response {:?}", rsp)
        }

        match parse_response(b"* OK [CAPABILITY IMAP4rev1 IDLE] Logged in\r\n") {
            Ok((_, Response::Data {
                status: Status::Ok,
                code: Some(ResponseCode::Capabilities(c)),
                information: Some("Logged in")
            })) => {
                assert_eq!(c.len(), 2);
                assert_eq!(c[0], "IMAP4rev1");
                assert_eq!(c[1], "IDLE");
            }
            rsp @ _ => panic!("unexpected response {:?}", rsp)
        }

        match parse_response(b"* OK [CAPABILITY UIDPLUS IMAP4rev1 IDLE] Logged in\r\n") {
            Ok((_, Response::Data {
                status: Status::Ok,
                code: Some(ResponseCode::Capabilities(c)),
                information: Some("Logged in")
            })) => {
                assert_eq!(c.len(), 3);
                assert_eq!(c[0], "UIDPLUS");
                assert_eq!(c[1], "IMAP4rev1");
                assert_eq!(c[2], "IDLE");
            }
            rsp @ _ => panic!("unexpected response {:?}", rsp)
        }

        // Missing IMAP4rev1
        match parse_response(b"* OK [CAPABILITY UIDPLUS IDLE] Logged in\r\n") {
            Ok((_, Response::Data {
                status: Status::Ok,
                code: None,
                information: Some("[CAPABILITY UIDPLUS IDLE] Logged in")
            })) => {}
            rsp @ _ => panic!("unexpected response {:?}", rsp)
        }

        match parse_response(b"* NO [BADCHARSET] error\r\n") {
            Ok((_, Response::Data {
                status: Status::No,
                code: Some(ResponseCode::BadCharset(None)),
                information: Some("error")
            })) => {},
            rsp @ _ => panic!("unexpected response {:?}", rsp)
        }

        match parse_response(b"* NO [BADCHARSET (utf-8 latin1)] error\r\n") {
            Ok((_, Response::Data {
                status: Status::No,
                code: Some(ResponseCode::BadCharset(Some(v))),
                information: Some("error")
            })) => {
                assert_eq!(v.len(), 2);
                assert_eq!(v[0], "utf-8");
                assert_eq!(v[1], "latin1");
            },
            rsp @ _ => panic!("unexpected response {:?}", rsp)
        }

        match parse_response(b"* NO [BADCHARSET ()] error\r\n") {
            Ok((_, Response::Data {
                status: Status::No,
                code: None,
                information: Some("[BADCHARSET ()] error")
            })) => {}
            rsp @ _ => panic!("unexpected response {:?}", rsp)
        }
    }
}
