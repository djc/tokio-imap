use nom::{self, IResult};

use std::str;

pub fn list_wildcards(c: u8) -> bool {
    c == b'%' || c == b'*'
}

pub fn quoted_specials(c: u8) -> bool {
    c == b'"' || c == b'\\'
}

pub fn resp_specials(c: u8) -> bool {
    c == b']'
}

pub fn atom_specials(c: u8) -> bool {
    c == b'('
        || c == b')'
        || c == b'{'
        || c == b' '
        || c < 32
        || list_wildcards(c)
        || quoted_specials(c)
        || resp_specials(c)
}

pub fn atom_char(c: u8) -> bool {
    !atom_specials(c)
}

pub fn astring_char(c: u8) -> bool {
    atom_char(c) || resp_specials(c)
}

pub fn quoted_data(i: &[u8]) -> IResult<&[u8], &[u8]> {
    // Ideally this should use nom's `escaped` macro, but it suffers from broken
    // type inference unless compiled with the verbose-errors feature enabled.
    let mut escape = false;
    let mut len = 0;
    for c in i {
        if *c == b'"' && !escape {
            break;
        }
        len += 1;
        if *c == b'\\' && !escape {
            escape = true
        } else if escape {
            escape = false;
        }
    }
    Ok((&i[len..], &i[..len]))
}

named!(pub quoted<&[u8]>, do_parse!(
    tag_s!("\"") >>
    data: quoted_data >>
    tag_s!("\"") >>
    (data)
));

named!(pub literal<&[u8]>, do_parse!(
    tag_s!("{") >>
    len: number >>
    tag_s!("}") >>
    tag_s!("\r\n") >>
    data: take!(len) >>
    (data)
));

named!(pub string<&[u8]>, alt!(quoted | literal));

named!(pub nstring<Option<&[u8]>>, map!(
    alt!(tag_s!("NIL") | string),
    |s| if s == b"NIL" { None } else { Some(s) }
));

named!(pub number<u32>, map_res!(
    map_res!(nom::digit, str::from_utf8),
    str::parse
));

named!(pub number_64<u64>, map_res!(
    map_res!(nom::digit, str::from_utf8),
    str::parse
));

named!(pub atom<&str>, map_res!(take_while1_s!(atom_char),
    str::from_utf8
));

named!(pub astring<&[u8]>, alt!(
    take_while1!(astring_char) |
    string
));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_literal() {
        match string(b"{3}\r\nXYZ") {
            Ok((_, value)) => {
                assert_eq!(value, "XYZ".as_bytes());
            }
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_astring() {
        match astring(b"text ") {
            Ok((_, value)) => {
                assert_eq!(value, "text".as_bytes());
            }
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }
}
