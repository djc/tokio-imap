use std::str;

use proto::{RequestId, Response};

pub fn parse(msg: &[u8]) -> Response {
    let (request_id, rest) = if msg[0] == b'*' {
        (None, &msg[2..])
    } else {
        let pos = msg.iter().position(|b| *b == b' ').unwrap();
        let tag = RequestId(str::from_utf8(&msg[..pos]).unwrap().to_string());
        (Some(tag), &msg[pos + 1..])
    };
    Response::Status(request_id, str::from_utf8(rest).unwrap().to_string())
}
