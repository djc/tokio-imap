use super::rfc3501::parse_response;
use crate::types::*;

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
fn test_uid_fetch_extra_space() {
    // DavMail inserts an extra space after RFC822.HEADER
    match parse_response(b"* 4 FETCH (UID 71372 RFC822.HEADER  {10275}\r\n") {
        Err(nom::Err::Incomplete(nom::Needed::Size(10275))) => {}
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
