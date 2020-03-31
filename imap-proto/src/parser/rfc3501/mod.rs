//!
//! https://tools.ietf.org/html/rfc3501
//!
//! INTERNET MESSAGE ACCESS PROTOCOL
//!

use std::str;

use nom::{
    self,
    branch::alt,
    bytes::streaming::{tag, tag_no_case, take_while, take_while1},
    character::streaming::char,
    combinator::{map, map_res, opt, recognize},
    multi::{many0, many1},
    sequence::{delimited, pair, preceded, terminated, tuple},
    IResult,
};

use crate::{
    parser::{
        core::*, rfc3501::body::*, rfc3501::body_structure::*, rfc4551, rfc5161,
        rfc5464::resp_metadata, ParseResult,
    },
    types::*,
};

pub mod body;
pub mod body_structure;

fn is_tag_char(c: u8) -> bool {
    c != b'+' && is_astring_char(c)
}

fn status_ok(i: &[u8]) -> IResult<&[u8], Status> {
    map(tag_no_case("OK"), |_s| Status::Ok)(i)
}
fn status_no(i: &[u8]) -> IResult<&[u8], Status> {
    map(tag_no_case("NO"), |_s| Status::No)(i)
}
fn status_bad(i: &[u8]) -> IResult<&[u8], Status> {
    map(tag_no_case("BAD"), |_s| Status::Bad)(i)
}
fn status_preauth(i: &[u8]) -> IResult<&[u8], Status> {
    map(tag_no_case("PREAUTH"), |_s| Status::PreAuth)(i)
}
fn status_bye(i: &[u8]) -> IResult<&[u8], Status> {
    map(tag_no_case("BYE"), |_s| Status::Bye)(i)
}

fn status(i: &[u8]) -> IResult<&[u8], Status> {
    alt((status_ok, status_no, status_bad, status_preauth, status_bye))(i)
}

fn mailbox(i: &[u8]) -> IResult<&[u8], &str> {
    map(astring_utf8, |s| {
        if s.eq_ignore_ascii_case("INBOX") {
            "INBOX"
        } else {
            s
        }
    })(i)
}

fn flag_extension(i: &[u8]) -> IResult<&[u8], &str> {
    map_res(
        recognize(pair(tag(b"\\"), take_while(is_atom_char))),
        str::from_utf8,
    )(i)
}

fn flag(i: &[u8]) -> IResult<&[u8], &str> {
    alt((flag_extension, atom))(i)
}

fn flag_list(i: &[u8]) -> IResult<&[u8], Vec<&str>> {
    parenthesized_list(flag)(i)
}

fn flag_perm(i: &[u8]) -> IResult<&[u8], &str> {
    alt((map_res(tag(b"\\*"), str::from_utf8), flag))(i)
}

fn resp_text_code_alert(i: &[u8]) -> IResult<&[u8], ResponseCode> {
    map(tag_no_case(b"ALERT"), |_| ResponseCode::Alert)(i)
}

fn resp_text_code_badcharset(i: &[u8]) -> IResult<&[u8], ResponseCode> {
    map(
        preceded(
            tag_no_case(b"BADCHARSET"),
            opt(preceded(
                tag(b" "),
                parenthesized_nonempty_list(astring_utf8),
            )),
        ),
        ResponseCode::BadCharset,
    )(i)
}

fn resp_text_code_capability(i: &[u8]) -> IResult<&[u8], ResponseCode> {
    map(capability_data, ResponseCode::Capabilities)(i)
}

fn resp_text_code_parse(i: &[u8]) -> IResult<&[u8], ResponseCode> {
    map(tag_no_case(b"PARSE"), |_| ResponseCode::Parse)(i)
}

fn resp_text_code_permanent_flags(i: &[u8]) -> IResult<&[u8], ResponseCode> {
    map(
        preceded(
            tag_no_case(b"PERMANENTFLAGS "),
            parenthesized_list(flag_perm),
        ),
        ResponseCode::PermanentFlags,
    )(i)
}

fn resp_text_code_read_only(i: &[u8]) -> IResult<&[u8], ResponseCode> {
    map(tag_no_case(b"READ-ONLY"), |_| ResponseCode::ReadOnly)(i)
}

fn resp_text_code_read_write(i: &[u8]) -> IResult<&[u8], ResponseCode> {
    map(tag_no_case(b"READ-WRITE"), |_| ResponseCode::ReadWrite)(i)
}

fn resp_text_code_try_create(i: &[u8]) -> IResult<&[u8], ResponseCode> {
    map(tag_no_case(b"TRYCREATE"), |_| ResponseCode::TryCreate)(i)
}

fn resp_text_code_uid_validity(i: &[u8]) -> IResult<&[u8], ResponseCode> {
    map(
        preceded(tag_no_case(b"UIDVALIDITY "), number),
        ResponseCode::UidValidity,
    )(i)
}

fn resp_text_code_uid_next(i: &[u8]) -> IResult<&[u8], ResponseCode> {
    map(
        preceded(tag_no_case(b"UIDNEXT "), number),
        ResponseCode::UidNext,
    )(i)
}

fn resp_text_code_unseen(i: &[u8]) -> IResult<&[u8], ResponseCode> {
    map(
        preceded(tag_no_case(b"UNSEEN "), number),
        ResponseCode::Unseen,
    )(i)
}

fn resp_text_code(i: &[u8]) -> IResult<&[u8], ResponseCode> {
    // Per the spec, the closing tag should be "] ".
    // See `resp_text` for more on why this is done differently.
    delimited(
        tag(b"["),
        alt((
            resp_text_code_alert,
            resp_text_code_badcharset,
            resp_text_code_capability,
            resp_text_code_parse,
            resp_text_code_permanent_flags,
            resp_text_code_uid_validity,
            resp_text_code_uid_next,
            resp_text_code_unseen,
            resp_text_code_read_only,
            resp_text_code_read_write,
            resp_text_code_try_create,
            rfc4551::resp_text_code_highest_mod_seq,
        )),
        tag(b"]"),
    )(i)
}

fn capability(i: &[u8]) -> IResult<&[u8], Capability> {
    alt((
        map(tag_no_case(b"IMAP4rev1"), |_| Capability::Imap4rev1),
        map(preceded(tag_no_case(b"AUTH="), atom), Capability::Auth),
        map(atom, Capability::Atom),
    ))(i)
}

fn ensure_capabilities_contains_imap4rev<'a>(
    capabilities: Vec<Capability<'a>>,
) -> Result<Vec<Capability<'a>>, ()> {
    if capabilities.contains(&Capability::Imap4rev1) {
        Ok(capabilities)
    } else {
        Err(())
    }
}

fn capability_data(i: &[u8]) -> IResult<&[u8], Vec<Capability>> {
    map_res(
        preceded(
            tag_no_case(b"CAPABILITY"),
            many0(preceded(char(' '), capability)),
        ),
        ensure_capabilities_contains_imap4rev,
    )(i)
}

fn resp_capability(i: &[u8]) -> IResult<&[u8], Response> {
    map(capability_data, Response::Capabilities)(i)
}

fn mailbox_data_search(i: &[u8]) -> IResult<&[u8], Response> {
    map(
        preceded(tag_no_case(b"SEARCH"), many0(preceded(tag(" "), number))),
        Response::IDs,
    )(i)
}

fn mailbox_data_flags(i: &[u8]) -> IResult<&[u8], Response> {
    map(preceded(tag_no_case("FLAGS "), flag_list), |flags| {
        Response::MailboxData(MailboxDatum::Flags(flags))
    })(i)
}

fn mailbox_data_exists(i: &[u8]) -> IResult<&[u8], Response> {
    map(terminated(number, tag_no_case(" EXISTS")), |num| {
        Response::MailboxData(MailboxDatum::Exists(num))
    })(i)
}

#[allow(clippy::type_complexity)]
fn mailbox_list(i: &[u8]) -> IResult<&[u8], (Vec<&str>, Option<&str>, &str)> {
    map(
        tuple((
            flag_list,
            tag(b" "),
            alt((map(quoted_utf8, Some), map(nil, |_| None))),
            tag(b" "),
            mailbox,
        )),
        |(flags, _, delimiter, _, name)| (flags, delimiter, name),
    )(i)
}

fn mailbox_data_list(i: &[u8]) -> IResult<&[u8], Response> {
    map(preceded(tag_no_case("LIST "), mailbox_list), |data| {
        Response::MailboxData(MailboxDatum::List {
            flags: data.0,
            delimiter: data.1,
            name: data.2,
        })
    })(i)
}

fn mailbox_data_lsub(i: &[u8]) -> IResult<&[u8], Response> {
    map(preceded(tag_no_case("LSUB "), mailbox_list), |data| {
        Response::MailboxData(MailboxDatum::List {
            flags: data.0,
            delimiter: data.1,
            name: data.2,
        })
    })(i)
}

// Unlike `status_att` in the RFC syntax, this includes the value,
// so that it can return a valid enum object instead of just a key.
fn status_att(i: &[u8]) -> IResult<&[u8], StatusAttribute> {
    alt((
        rfc4551::status_att_val_highest_mod_seq,
        map(
            preceded(tag_no_case("MESSAGES "), number),
            StatusAttribute::Messages,
        ),
        map(
            preceded(tag_no_case("RECENT "), number),
            StatusAttribute::Recent,
        ),
        map(
            preceded(tag_no_case("UIDNEXT "), number),
            StatusAttribute::UidNext,
        ),
        map(
            preceded(tag_no_case("UIDVALIDITY "), number),
            StatusAttribute::UidValidity,
        ),
        map(
            preceded(tag_no_case("UNSEEN "), number),
            StatusAttribute::Unseen,
        ),
    ))(i)
}

fn status_att_list(i: &[u8]) -> IResult<&[u8], Vec<StatusAttribute>> {
    parenthesized_nonempty_list(status_att)(i)
}

fn mailbox_data_status(i: &[u8]) -> IResult<&[u8], Response> {
    map(
        tuple((tag_no_case("STATUS "), mailbox, tag(" "), status_att_list)),
        |(_, mailbox, _, status)| Response::MailboxData(MailboxDatum::Status { mailbox, status }),
    )(i)
}

fn mailbox_data_recent(i: &[u8]) -> IResult<&[u8], Response> {
    map(terminated(number, tag_no_case(" RECENT")), |num| {
        Response::MailboxData(MailboxDatum::Recent(num))
    })(i)
}

fn mailbox_data(i: &[u8]) -> IResult<&[u8], Response> {
    alt((
        mailbox_data_flags,
        mailbox_data_exists,
        mailbox_data_list,
        mailbox_data_lsub,
        mailbox_data_status,
        mailbox_data_recent,
        mailbox_data_search,
    ))(i)
}

// An address structure is a parenthesized list that describes an
// electronic mail address.
fn address(i: &[u8]) -> IResult<&[u8], Address> {
    paren_delimited(map(
        tuple((
            nstring,
            tag(" "),
            nstring,
            tag(" "),
            nstring,
            tag(" "),
            nstring,
        )),
        |(name, _, adl, _, mailbox, _, host)| Address {
            name,
            adl,
            mailbox,
            host,
        },
    ))(i)
}

fn opt_addresses(i: &[u8]) -> IResult<&[u8], Option<Vec<Address>>> {
    alt((
        map(nil, |_s| None),
        map(
            paren_delimited(many1(terminated(address, opt(char(' '))))),
            Some,
        ),
    ))(i)
}

pub(crate) fn envelope(i: &[u8]) -> IResult<&[u8], Envelope> {
    paren_delimited(map(
        tuple((
            nstring,
            tag(" "),
            nstring,
            tag(" "),
            opt_addresses,
            tag(" "),
            opt_addresses,
            tag(" "),
            opt_addresses,
            tag(" "),
            opt_addresses,
            tag(" "),
            opt_addresses,
            tag(" "),
            opt_addresses,
            tag(" "),
            nstring,
            tag(" "),
            nstring,
        )),
        |(
            date,
            _,
            subject,
            _,
            from,
            _,
            sender,
            _,
            reply_to,
            _,
            to,
            _,
            cc,
            _,
            bcc,
            _,
            in_reply_to,
            _,
            message_id,
        )| Envelope {
            date,
            subject,
            from,
            sender,
            reply_to,
            to,
            cc,
            bcc,
            in_reply_to,
            message_id,
        },
    ))(i)
}

fn msg_att_envelope(i: &[u8]) -> IResult<&[u8], AttributeValue> {
    map(preceded(tag_no_case("ENVELOPE "), envelope), |envelope| {
        AttributeValue::Envelope(Box::new(envelope))
    })(i)
}

fn msg_att_internal_date(i: &[u8]) -> IResult<&[u8], AttributeValue> {
    map(
        preceded(tag_no_case("INTERNALDATE "), nstring_utf8),
        |date| AttributeValue::InternalDate(date.unwrap()),
    )(i)
}

fn msg_att_flags(i: &[u8]) -> IResult<&[u8], AttributeValue> {
    map(
        preceded(tag_no_case("FLAGS "), flag_list),
        AttributeValue::Flags,
    )(i)
}

fn msg_att_rfc822(i: &[u8]) -> IResult<&[u8], AttributeValue> {
    map(
        preceded(tag_no_case("RFC822 "), nstring),
        AttributeValue::Rfc822,
    )(i)
}

fn msg_att_rfc822_header(i: &[u8]) -> IResult<&[u8], AttributeValue> {
    // extra space workaround for DavMail
    map(
        tuple((tag_no_case("RFC822.HEADER "), opt(tag(b" ")), nstring)),
        |(_, _, raw)| AttributeValue::Rfc822Header(raw),
    )(i)
}

fn msg_att_rfc822_size(i: &[u8]) -> IResult<&[u8], AttributeValue> {
    map(
        preceded(tag_no_case("RFC822.SIZE "), number),
        AttributeValue::Rfc822Size,
    )(i)
}

fn msg_att_rfc822_text(i: &[u8]) -> IResult<&[u8], AttributeValue> {
    map(
        preceded(tag_no_case("RFC822.TEXT "), nstring),
        AttributeValue::Rfc822Text,
    )(i)
}

fn msg_att_uid(i: &[u8]) -> IResult<&[u8], AttributeValue> {
    map(preceded(tag_no_case("UID "), number), AttributeValue::Uid)(i)
}

fn msg_att(i: &[u8]) -> IResult<&[u8], AttributeValue> {
    alt((
        msg_att_body_section,
        msg_att_body_structure,
        msg_att_envelope,
        msg_att_internal_date,
        msg_att_flags,
        rfc4551::msg_att_mod_seq,
        msg_att_rfc822,
        msg_att_rfc822_header,
        msg_att_rfc822_size,
        msg_att_rfc822_text,
        msg_att_uid,
    ))(i)
}

fn msg_att_list(i: &[u8]) -> IResult<&[u8], Vec<AttributeValue>> {
    parenthesized_nonempty_list(msg_att)(i)
}

fn message_data_fetch(i: &[u8]) -> IResult<&[u8], Response> {
    map(
        tuple((number, tag_no_case(" FETCH "), msg_att_list)),
        |(num, _, attrs)| Response::Fetch(num, attrs),
    )(i)
}

fn message_data_expunge(i: &[u8]) -> IResult<&[u8], Response> {
    map(
        terminated(number, tag_no_case(" EXPUNGE")),
        Response::Expunge,
    )(i)
}

fn imap_tag(i: &[u8]) -> IResult<&[u8], RequestId> {
    map(map_res(take_while1(is_tag_char), str::from_utf8), |s| {
        RequestId(s.to_string())
    })(i)
}

// This is not quite according to spec, which mandates the following:
//     ["[" resp-text-code "]" SP] text
// However, examples in RFC 4551 (Conditional STORE) counteract this by giving
// examples of `resp-text` that do not include the trailing space and text.
fn resp_text(i: &[u8]) -> IResult<&[u8], (Option<ResponseCode>, Option<&str>)> {
    map(tuple((opt(resp_text_code), text)), |(code, text)| {
        let res = if text.is_empty() {
            None
        } else if code.is_some() {
            Some(&text[1..])
        } else {
            Some(text)
        };
        (code, res)
    })(i)
}

fn continue_req(i: &[u8]) -> IResult<&[u8], Response> {
    // Some servers do not send the space :/
    // TODO: base64
    map(
        tuple((tag("+"), opt(tag(" ")), resp_text, tag("\r\n"))),
        |(_, _, text, _)| Response::Continue {
            code: text.0,
            information: text.1,
        },
    )(i)
}

fn response_tagged(i: &[u8]) -> IResult<&[u8], Response> {
    map(
        tuple((
            imap_tag,
            tag(b" "),
            status,
            tag(b" "),
            resp_text,
            tag(b"\r\n"),
        )),
        |(tag, _, status, _, text, _)| Response::Done {
            tag,
            status,
            code: text.0,
            information: text.1,
        },
    )(i)
}

fn resp_cond(i: &[u8]) -> IResult<&[u8], Response> {
    map(
        tuple((status, tag(b" "), resp_text)),
        |(status, _, text)| Response::Data {
            status,
            code: text.0,
            information: text.1,
        },
    )(i)
}

fn response_data(i: &[u8]) -> IResult<&[u8], Response> {
    delimited(
        tag(b"* "),
        alt((
            resp_cond,
            mailbox_data,
            message_data_expunge,
            message_data_fetch,
            resp_capability,
            resp_metadata,
            rfc5161::resp_enabled,
        )),
        tag(b"\r\n"),
    )(i)
}

fn response(i: &[u8]) -> IResult<&[u8], Response> {
    alt((continue_req, response_data, response_tagged))(i)
}

pub fn parse_response(msg: &[u8]) -> ParseResult {
    response(msg)
}

#[cfg(test)]
mod tests {
    use super::parse_response;
    use crate::types::*;
    use nom;

    #[test]
    fn test_number_overflow() {
        match parse_response(b"* 2222222222222222222222222222222222222222222C\r\n") {
            Err(_) => {}
            _ => panic!("error required for integer overflow"),
        }
    }

    #[test]
    fn test_unseen() {
        match parse_response(b"* OK [UNSEEN 3] Message 3 is first unseen\r\n").unwrap() {
            (
                _,
                Response::Data {
                    status: Status::Ok,
                    code: Some(ResponseCode::Unseen(3)),
                    information: Some("Message 3 is first unseen"),
                },
            ) => {}
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_body_text() {
        match parse_response(b"* 2 FETCH (BODY[TEXT] {3}\r\nfoo)\r\n") {
            Ok((_, Response::Fetch(_, attrs))) => {
                let body = &attrs[0];
                assert_eq!(
                    body,
                    &AttributeValue::BodySection {
                        section: Some(SectionPath::Full(MessageSection::Text)),
                        index: None,
                        data: Some(b"foo"),
                    },
                    "body = {:?}",
                    body
                );
            }
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_body_structure() {
        const RESPONSE: &[u8] = b"* 15 FETCH (BODYSTRUCTURE (\"TEXT\" \"PLAIN\" (\"CHARSET\" \"iso-8859-1\") NIL NIL \"QUOTED-PRINTABLE\" 1315 42 NIL NIL NIL NIL))\r\n";
        match parse_response(RESPONSE) {
            Ok((_, Response::Fetch(_, attrs))) => {
                let body = &attrs[0];
                assert!(
                    if let AttributeValue::BodyStructure(_) = *body {
                        true
                    } else {
                        false
                    },
                    "body = {:?}",
                    body
                );
            }
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_status() {
        match parse_response(b"* STATUS blurdybloop (MESSAGES 231 UIDNEXT 44292)\r\n") {
            Ok((_, Response::MailboxData(MailboxDatum::Status { mailbox, status }))) => {
                assert_eq!(mailbox, "blurdybloop");
                assert_eq!(
                    status,
                    [
                        StatusAttribute::Messages(231),
                        StatusAttribute::UidNext(44292),
                    ]
                );
            }
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_notify() {
        match parse_response(b"* 3501 EXPUNGE\r\n") {
            Ok((_, Response::Expunge(3501))) => {}
            rsp => panic!("unexpected response {:?}", rsp),
        }
        match parse_response(b"* 3501 EXISTS\r\n") {
            Ok((_, Response::MailboxData(MailboxDatum::Exists(3501)))) => {}
            rsp => panic!("unexpected response {:?}", rsp),
        }
        match parse_response(b"+ idling\r\n") {
            Ok((
                _,
                Response::Continue {
                    code: None,
                    information: Some("idling"),
                },
            )) => {}
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_search() {
        match parse_response(b"* SEARCH\r\n") {
            Ok((_, Response::IDs(ids))) => {
                assert!(ids.is_empty());
            }
            rsp => panic!("unexpected response {:?}", rsp),
        }
        match parse_response(b"* SEARCH 12345 67890\r\n") {
            Ok((_, Response::IDs(ids))) => {
                assert_eq!(ids[0], 12345);
                assert_eq!(ids[1], 67890);
            }
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_uid_fetch() {
        match parse_response(b"* 4 FETCH (UID 71372 RFC822.HEADER {10275}\r\n") {
            Err(nom::Err::Incomplete(nom::Needed::Size(10275))) => {}
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_list() {
        match crate::parser::rfc3501::mailbox(b"iNboX ") {
            Ok((_, mb)) => {
                assert_eq!(mb, "INBOX");
            }
            rsp => panic!("unexpected response {:?}", rsp),
        }

        match parse_response(b"* LIST (\\HasNoChildren) \".\" INBOX.Tests\r\n") {
            Ok((_, Response::MailboxData(_))) => {}
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_uid_fetch_extra_space() {
        // DavMail inserts an extra space after RFC822.HEADER
        match parse_response(b"* 4 FETCH (UID 71372 RFC822.HEADER  {10275}\r\n") {
            Err(nom::Err::Incomplete(nom::Needed::Size(10275))) => {}
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_envelope() {
        let env = br#"ENVELOPE ("Wed, 17 Jul 1996 02:23:25 -0700 (PDT)" "IMAP4rev1 WG mtg summary and minutes" (("Terry Gray" NIL "gray" "cac.washington.edu")) (("Terry Gray" NIL "gray" "cac.washington.edu")) (("Terry Gray" NIL "gray" "cac.washington.edu")) ((NIL NIL "imap" "cac.washington.edu")) ((NIL NIL "minutes" "CNRI.Reston.VA.US") ("John Klensin" NIL "KLENSIN" "MIT.EDU")) NIL NIL "<B27397-0100000@cac.washington.edu>") "#;
        match crate::parser::rfc3501::msg_att_envelope(env) {
            Ok((_, AttributeValue::Envelope(_))) => {}
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_header_fields() {
        const RESPONSE: &[u8] = b"* 1 FETCH (UID 1 BODY[HEADER.FIELDS (CHAT-VERSION)] {21}\r\nChat-Version: 1.0\r\n\r\n)\r\n";

        match parse_response(RESPONSE) {
            Ok((_, Response::Fetch(_, _))) => {}
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_opt_addresses() {
        let addr = b"((NIL NIL \"minutes\" \"CNRI.Reston.VA.US\") (\"John Klensin\" NIL \"KLENSIN\" \"MIT.EDU\")) ";
        match crate::parser::rfc3501::opt_addresses(addr) {
            Ok((_, _addresses)) => {}
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_opt_addresses_no_space() {
        let addr =
            br#"((NIL NIL "test" "example@example.com")(NIL NIL "test" "example@example.com"))"#;
        match super::opt_addresses(addr) {
            Ok((_, _addresses)) => {}
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_addresses() {
        match crate::parser::rfc3501::address(b"(\"John Klensin\" NIL \"KLENSIN\" \"MIT.EDU\") ") {
            Ok((_, _address)) => {}
            rsp => panic!("unexpected response {:?}", rsp),
        }

        // Literal non-UTF8 address
        match crate::parser::rfc3501::address(
            b"({12}\r\nJoh\xff Klensin NIL \"KLENSIN\" \"MIT.EDU\") ",
        ) {
            Ok((_, _address)) => {}
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_response_codes() {
        match parse_response(b"* OK [ALERT] Alert!\r\n") {
            Ok((
                _,
                Response::Data {
                    status: Status::Ok,
                    code: Some(ResponseCode::Alert),
                    information: Some("Alert!"),
                },
            )) => {}
            rsp => panic!("unexpected response {:?}", rsp),
        }

        match parse_response(b"* NO [PARSE] Something\r\n") {
            Ok((
                _,
                Response::Data {
                    status: Status::No,
                    code: Some(ResponseCode::Parse),
                    information: Some("Something"),
                },
            )) => {}
            rsp => panic!("unexpected response {:?}", rsp),
        }

        match parse_response(b"* OK [CAPABILITY IMAP4rev1 IDLE] Logged in\r\n") {
            Ok((
                _,
                Response::Data {
                    status: Status::Ok,
                    code: Some(ResponseCode::Capabilities(c)),
                    information: Some("Logged in"),
                },
            )) => {
                assert_eq!(c.len(), 2);
                assert_eq!(c[0], Capability::Imap4rev1);
                assert_eq!(c[1], Capability::Atom("IDLE"));
            }
            rsp => panic!("unexpected response {:?}", rsp),
        }

        match parse_response(b"* OK [CAPABILITY UIDPLUS IMAP4rev1 IDLE] Logged in\r\n") {
            Ok((
                _,
                Response::Data {
                    status: Status::Ok,
                    code: Some(ResponseCode::Capabilities(c)),
                    information: Some("Logged in"),
                },
            )) => {
                assert_eq!(c.len(), 3);
                assert_eq!(c[0], Capability::Atom("UIDPLUS"));
                assert_eq!(c[1], Capability::Imap4rev1);
                assert_eq!(c[2], Capability::Atom("IDLE"));
            }
            rsp => panic!("unexpected response {:?}", rsp),
        }

        // Missing IMAP4rev1
        match parse_response(b"* OK [CAPABILITY UIDPLUS IDLE] Logged in\r\n") {
            Ok((
                _,
                Response::Data {
                    status: Status::Ok,
                    code: None,
                    information: Some("[CAPABILITY UIDPLUS IDLE] Logged in"),
                },
            )) => {}
            rsp => panic!("unexpected response {:?}", rsp),
        }

        match parse_response(b"* NO [BADCHARSET] error\r\n") {
            Ok((
                _,
                Response::Data {
                    status: Status::No,
                    code: Some(ResponseCode::BadCharset(None)),
                    information: Some("error"),
                },
            )) => {}
            rsp => panic!("unexpected response {:?}", rsp),
        }

        match parse_response(b"* NO [BADCHARSET (utf-8 latin1)] error\r\n") {
            Ok((
                _,
                Response::Data {
                    status: Status::No,
                    code: Some(ResponseCode::BadCharset(Some(v))),
                    information: Some("error"),
                },
            )) => {
                assert_eq!(v.len(), 2);
                assert_eq!(v[0], "utf-8");
                assert_eq!(v[1], "latin1");
            }
            rsp => panic!("unexpected response {:?}", rsp),
        }

        match parse_response(b"* NO [BADCHARSET ()] error\r\n") {
            Ok((
                _,
                Response::Data {
                    status: Status::No,
                    code: None,
                    information: Some("[BADCHARSET ()] error"),
                },
            )) => {}
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_capability_data() {
        // Minimal capabilities
        assert_matches!(
            super::capability_data(b"CAPABILITY IMAP4rev1\r\n"),
            Ok((_, capabilities)) => {
                assert_eq!(capabilities, vec![Capability::Imap4rev1])
            }
        );

        assert_matches!(
            super::capability_data(b"CAPABILITY IMAP4REV1\r\n"),
            Ok((_, capabilities)) => {
                assert_eq!(capabilities, vec![Capability::Imap4rev1])
            }
        );

        assert_matches!(
            super::capability_data(b"CAPABILITY XPIG-LATIN IMAP4rev1 STARTTLS AUTH=GSSAPI\r\n"),
            Ok((_, capabilities)) => {
                assert_eq!(capabilities, vec![
                    Capability::Atom("XPIG-LATIN"), Capability::Imap4rev1,
                    Capability::Atom("STARTTLS"), Capability::Auth("GSSAPI")
                ])
            }
        );

        assert_matches!(
            super::capability_data(b"CAPABILITY IMAP4rev1 AUTH=GSSAPI AUTH=PLAIN\r\n"),
            Ok((_, capabilities)) => {
                assert_eq!(capabilities, vec![
                    Capability::Imap4rev1, Capability::Auth("GSSAPI"),  Capability::Auth("PLAIN")
                ])
            }
        );

        // Capability command must contain IMAP4rev1
        assert_matches!(
            super::capability_data(b"CAPABILITY AUTH=GSSAPI AUTH=PLAIN\r\n"),
            Err(_)
        );
    }

    #[test]
    fn test_incomplete_fetch() {
        match parse_response(b"* 4644 FETCH (UID ") {
            Err(nom::Err::Incomplete(_)) => {}
            rsp => panic!("should be incomplete: {:?}", rsp),
        }
    }

    #[test]
    fn test_continuation() {
        // regular RFC compliant
        match parse_response(b"+ \r\n") {
            Ok((
                _,
                Response::Continue {
                    code: None,
                    information: None,
                },
            )) => {}
            rsp => panic!("unexpected response {:?}", rsp),
        }

        // short version, sent by yandex
        match parse_response(b"+\r\n") {
            Ok((
                _,
                Response::Continue {
                    code: None,
                    information: None,
                },
            )) => {}
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_enabled() {
        match parse_response(b"* ENABLED QRESYNC X-GOOD-IDEA\r\n") {
            Ok((_, capabilities)) => assert_eq!(
                capabilities,
                Response::Capabilities(vec![
                    Capability::Atom("QRESYNC"),
                    Capability::Atom("X-GOOD-IDEA"),
                ])
            ),
            rsp => panic!("Unexpected response: {:?}", rsp),
        }
    }
}
