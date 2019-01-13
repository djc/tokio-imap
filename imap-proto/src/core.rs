use nom::{self, IResult};

use std::str;

/// list-wildcards = "%" / "*"
pub fn list_wildcards(c: u8) -> bool {
    c == b'%' || c == b'*'
}

/// quoted-specials = DQUOTE / "\"
pub fn quoted_specials(c: u8) -> bool {
    c == b'"' || c == b'\\'
}

/// resp-specials = "]"
pub fn resp_specials(c: u8) -> bool {
    c == b']'
}

/// atom-specials = "(" / ")" / "{" / SP / CTL / list-wildcards / quoted-specials / resp-specials
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

/// ATOM-CHAR = <any CHAR except atom-specials>
pub fn atom_char(c: u8) -> bool {
    !atom_specials(c)
}

/// ASTRING-CHAR = ATOM-CHAR / resp-specials
pub fn astring_char(c: u8) -> bool {
    atom_char(c) || resp_specials(c)
}

/// QUOTED-CHAR = <any TEXT-CHAR except quoted-specials> / "\" quoted-specials
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

/// quoted = DQUOTE *QUOTED-CHAR DQUOTE
named!(pub quoted<&[u8]>, do_parse!(
    tag_s!("\"") >>
    data: quoted_data >>
    tag_s!("\"") >>
    (data)
));

/// literal = "{" number "}" CRLF *CHAR8
///            ; Number represents the number of CHAR8s
named!(pub literal<&[u8]>, do_parse!(
    tag_s!("{") >>
    len: number >>
    tag_s!("}") >>
    tag_s!("\r\n") >>
    data: take!(len) >>
    (data)
));

/// string = quoted / literal
named!(pub string<&[u8]>, alt!(quoted | literal));

/// nstring = string / nil
named!(pub nstring<Option<&[u8]>>, map!(
    alt!(tag_s!("NIL") | string),
    |s| if s == b"NIL" { None } else { Some(s) }
));

/// number          = 1*DIGIT
///                    ; Unsigned 32-bit integer
///                    ; (0 <= n < 4,294,967,296)
named!(pub number<u32>, map_res!(
    map_res!(nom::digit, str::from_utf8),
    str::parse
));

/// same as `number` but 64-bit
named!(pub number_64<u64>, map_res!(
    map_res!(nom::digit, str::from_utf8),
    str::parse
));

/// atom = 1*ATOM-CHAR
named!(pub atom<&str>, map_res!(take_while1_s!(atom_char),
    str::from_utf8
));

/// astring = 1*ASTRING-CHAR / string
named!(pub astring<&[u8]>, alt!(
    take_while1!(astring_char) |
    string
));

/// text = 1*TEXT-CHAR
named!(pub text<&str>, map_res!(take_till_s!(crlf),
    str::from_utf8
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
