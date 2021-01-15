use super::{bodystructure::BodyStructParser, parse_response};
use crate::types::*;
use std::num::NonZeroUsize;

#[test]
fn test_mailbox_data_response() {
    match parse_response(b"* LIST (\\HasNoChildren) \".\" INBOX.Tests\r\n") {
        Ok((_, Response::MailboxData(_))) => {}
        rsp => panic!("unexpected response {:?}", rsp),
    }
}

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
                matches!(*body, AttributeValue::BodyStructure(_)),
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
    // also allow trailing whitespace in SEARCH responses
    for empty_response in &["* SEARCH\r\n", "* SEARCH \r\n"] {
        match parse_response(empty_response.as_bytes()) {
            Ok((_, Response::MailboxData(MailboxDatum::Search(ids)))) => {
                assert!(ids.is_empty());
            }
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }
    for response in &["* SEARCH 12345 67890\r\n", "* SEARCH 12345 67890 \r\n"] {
        match parse_response(response.as_bytes()) {
            Ok((_, Response::MailboxData(MailboxDatum::Search(ids)))) => {
                assert_eq!(ids[0], 12345);
                assert_eq!(ids[1], 67890);
            }
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }
}

#[test]
fn test_uid_fetch() {
    match parse_response(b"* 4 FETCH (UID 71372 RFC822.HEADER {10275}\r\n") {
        Err(nom::Err::Incomplete(nom::Needed::Size(size))) => {
            assert_eq!(size, NonZeroUsize::new(10275).unwrap());
        }
        rsp => panic!("unexpected response {:?}", rsp),
    }
}

#[test]
fn test_uid_fetch_extra_space() {
    // DavMail inserts an extra space after RFC822.HEADER
    match parse_response(b"* 4 FETCH (UID 71372 RFC822.HEADER  {10275}\r\n") {
        Err(nom::Err::Incomplete(nom::Needed::Size(size))) => {
            assert_eq!(size, NonZeroUsize::new(10275).unwrap());
        }
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

#[test]
fn test_flags() {
    // Invalid response (FLAGS can't include \*) from Zoho Mail server.
    //
    // As a workaround, such response is parsed without error.
    match parse_response(b"* FLAGS (\\Answered \\Flagged \\Deleted \\Seen \\Draft \\*)\r\n") {
        Ok((_, capabilities)) => assert_eq!(
            capabilities,
            Response::MailboxData(MailboxDatum::Flags(vec![
                "\\Answered",
                "\\Flagged",
                "\\Deleted",
                "\\Seen",
                "\\Draft",
                "\\*"
            ]))
        ),
        rsp => panic!("Unexpected response: {:?}", rsp),
    }
}

#[test]
fn test_vanished() {
    match parse_response(b"* VANISHED (EARLIER) 1,2,3:8\r\n") {
        Ok((_, Response::Vanished { earlier, uids })) => {
            assert_eq!(earlier, true);
            assert_eq!(uids.len(), 3);
            let v = &uids[0];
            assert_eq!(*v.start(), 1);
            assert_eq!(*v.end(), 1);
            let v = &uids[1];
            assert_eq!(*v.start(), 2);
            assert_eq!(*v.end(), 2);
            let v = &uids[2];
            assert_eq!(*v.start(), 3);
            assert_eq!(*v.end(), 8);
        }
        rsp => panic!("Unexpected response: {:?}", rsp),
    }

    match parse_response(b"* VANISHED 1,2,3:8,10\r\n") {
        Ok((_, Response::Vanished { earlier, uids })) => {
            assert_eq!(earlier, false);
            assert_eq!(uids.len(), 4);
        }
        rsp => panic!("Unexpected response: {:?}", rsp),
    }

    match parse_response(b"* VANISHED (EARLIER) 1\r\n") {
        Ok((_, Response::Vanished { earlier, uids })) => {
            assert_eq!(earlier, true);
            assert_eq!(uids.len(), 1);
            assert_eq!(uids[0].clone().collect::<Vec<u32>>(), vec![1]);
        }
        rsp => panic!("Unexpected response: {:?}", rsp),
    }

    match parse_response(b"* VANISHED 1\r\n") {
        Ok((_, Response::Vanished { earlier, uids })) => {
            assert_eq!(earlier, false);
            assert_eq!(uids.len(), 1);
        }
        rsp => panic!("Unexpected response: {:?}", rsp),
    }

    assert!(parse_response(b"* VANISHED \r\n").is_err());
    assert!(parse_response(b"* VANISHED (EARLIER) \r\n").is_err());
}

#[test]
fn test_uidplus() {
    match dbg!(parse_response(
        b"* OK [APPENDUID 38505 3955] APPEND completed\r\n"
    )) {
        Ok((
            _,
            Response::Data {
                status: Status::Ok,
                code: Some(ResponseCode::AppendUid(38505, uid_set)),
                information: Some("APPEND completed"),
            },
        )) if uid_set == [3955.into()] => {}
        rsp => panic!("Unexpected response: {:?}", rsp),
    }
    match dbg!(parse_response(
        b"* OK [COPYUID 38505 304,319:320 3956:3958] Done\r\n"
    )) {
        Ok((
            _,
            Response::Data {
                status: Status::Ok,
                code: Some(ResponseCode::CopyUid(38505, uid_set_src, uid_set_dst)),
                information: Some("Done"),
            },
        )) if uid_set_src == [304.into(), (319..=320).into()]
            && uid_set_dst == [(3956..=3958).into()] => {}
        rsp => panic!("Unexpected response: {:?}", rsp),
    }
    match dbg!(parse_response(
        b"* NO [UIDNOTSTICKY] Non-persistent UIDs\r\n"
    )) {
        Ok((
            _,
            Response::Data {
                status: Status::No,
                code: Some(ResponseCode::UidNotSticky),
                information: Some("Non-persistent UIDs"),
            },
        )) => {}
        rsp => panic!("Unexpected response: {:?}", rsp),
    }
}

#[test]
fn test_imap_body_structure() {
    let test = b"\
    * 1569 FETCH (\
        BODYSTRUCTURE (\
            (\
                (\
                    (\
                        \"TEXT\" \"PLAIN\" \
                        (\"CHARSET\" \"ISO-8859-1\") NIL NIL \
                        \"QUOTED-PRINTABLE\" 833 30 NIL NIL NIL\
                    )\
                    (\
                        \"TEXT\" \"HTML\" \
                        (\"CHARSET\" \"ISO-8859-1\") NIL NIL \
                        \"QUOTED-PRINTABLE\" 3412 62 NIL \
                        (\"INLINE\" NIL) NIL\
                    ) \
                    \"ALTERNATIVE\" (\"BOUNDARY\" \"2__=fgrths\") NIL NIL\
                )\
                (\
                    \"IMAGE\" \"GIF\" \
                    (\"NAME\" \"485039.gif\") \"<2__=lgkfjr>\" NIL \
                    \"BASE64\" 64 NIL (\"INLINE\" (\"FILENAME\" \"485039.gif\")) \
                    NIL\
                ) \
                \"RELATED\" (\"BOUNDARY\" \"1__=fgrths\") NIL NIL\
            )\
            (\
                \"APPLICATION\" \"PDF\" \
                (\"NAME\" \"title.pdf\") \
                \"<1__=lgkfjr>\" NIL \"BASE64\" 333980 NIL \
                (\"ATTACHMENT\" (\"FILENAME\" \"title.pdf\")) NIL\
            ) \
            \"MIXED\" (\"BOUNDARY\" \"0__=fgrths\") NIL NIL\
        )\
    )\r\n";

    let (_, resp) = parse_response(test).unwrap();
    match resp {
        Response::Fetch(_, f) => {
            let bodystructure = f
                .iter()
                .flat_map(|f| match f {
                    AttributeValue::BodyStructure(e) => Some(e),
                    _ => None,
                })
                .next()
                .unwrap();

            let parser = BodyStructParser::new(bodystructure);

            let element = parser.search(|b: &BodyStructure| {
                matches!(b, BodyStructure::Basic { ref common, .. } if common.ty.ty == "APPLICATION")
            });

            assert_eq!(element, Some(vec![2]));
        }
        _ => panic!("invalid FETCH command test"),
    };
}
