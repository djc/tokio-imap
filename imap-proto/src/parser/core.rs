use nom::{character::streaming::digit1, IResult};

// ----- number -----

// number          = 1*DIGIT
//                    ; Unsigned 32-bit integer
//                    ; (0 <= n < 4,294,967,296)
named!(pub number<u32>, flat_map!(digit1, parse_to!(u32)));

// same as `number` but 64-bit
named!(pub number_64<u64>, flat_map!(digit1, parse_to!(u64)));

// ----- string -----

// string = quoted / literal
named!(pub string<&[u8]>, alt!(quoted | literal));

// string bytes as utf8
named!(pub string_utf8<&str>, map_res!(string, std::str::from_utf8));

// quoted = DQUOTE *QUOTED-CHAR DQUOTE
named!(pub quoted<&[u8]>, delimited!(
    char!('"'),
    quoted_data,
    char!('"')
));

// quoted bytes as as utf8
named!(pub quoted_utf8<&str>, map_res!(quoted, std::str::from_utf8));

// QUOTED-CHAR = <any TEXT-CHAR except quoted-specials> / "\" quoted-specials
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

// quoted-specials = DQUOTE / "\"
pub fn is_quoted_specials(c: u8) -> bool {
    c == b'"' || c == b'\\'
}

// literal = "{" number "}" CRLF *CHAR8
//            ; Number represents the number of CHAR8s
named!(pub literal<&[u8]>, do_parse!(
    tag!("{") >>
    len: number >>
    tag!("}") >>
    tag!("\r\n") >>
    data: take!(len) >> // FIXME: 0x00 is not allowed
    (data)
));

// ----- astring ----- atom (roughly) or string

// astring = 1*ASTRING-CHAR / string
named!(pub astring<&[u8]>, alt!(
    take_while1!(is_astring_char) |
    string
));

// astring bytes as utf8
named!(pub astring_utf8<&str>, map_res!(astring, std::str::from_utf8));

// ASTRING-CHAR = ATOM-CHAR / resp-specials
pub fn is_astring_char(c: u8) -> bool {
    is_atom_char(c) || is_resp_specials(c)
}

// ATOM-CHAR = <any CHAR except atom-specials>
pub fn is_atom_char(c: u8) -> bool {
    !is_atom_specials(c)
}

// atom-specials = "(" / ")" / "{" / SP / CTL / list-wildcards / quoted-specials / resp-specials
pub fn is_atom_specials(c: u8) -> bool {
    c == b'('
        || c == b')'
        || c == b'{'
        || c == b' '
        || c < 32
        || is_list_wildcards(c)
        || is_quoted_specials(c)
        || is_resp_specials(c)
}

// resp-specials = "]"
pub fn is_resp_specials(c: u8) -> bool {
    c == b']'
}

// atom = 1*ATOM-CHAR
named!(pub atom<&str>, map_res!(take_while1!(is_atom_char),
    std::str::from_utf8
));

// ----- nstring ----- nil or string

// nstring = string / nil
named!(pub nstring<Option<&[u8]>>, alt!(
    map!(nil, |_| None) |
    map!(string, |s| Some(s))
));

// nstring bytes as utf8
named!(pub nstring_utf8<Option<&str>>, alt!(
    map!(nil, |_| None) |
    map!(string_utf8, |s| Some(s))
));

// nil = "NIL"
named!(pub nil, tag_no_case!("NIL"));

// ----- text -----

// text = 1*TEXT-CHAR
named!(pub text<&str>, map_res!(take_while!(is_text_char),
    std::str::from_utf8
));

// TEXT-CHAR = <any CHAR except CR and LF>
pub fn is_text_char(c: u8) -> bool {
    c != b'\r' && c != b'\n'
}

// ----- others -----

// list-wildcards = "%" / "*"
pub fn is_list_wildcards(c: u8) -> bool {
    c == b'%' || c == b'*'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_literal() {
        match string(b"{3}\r\nXYZ") {
            Ok((_, value)) => {
                assert_eq!(value, b"XYZ");
            }
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_astring() {
        match astring(b"text ") {
            Ok((_, value)) => {
                assert_eq!(value, b"text");
            }
            rsp => panic!("unexpected response {:?}", rsp),
        }
    }
}
