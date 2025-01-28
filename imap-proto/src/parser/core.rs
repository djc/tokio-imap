use nom::{
    branch::alt,
    bytes::streaming::{escaped, tag, tag_no_case, take, take_while, take_while1},
    character::streaming::{char, digit1, one_of},
    combinator::{map, map_res, opt},
    multi::{separated_list0, separated_list1},
    sequence::{delimited, preceded, tuple},
    IResult,
};

use std::str::{from_utf8, FromStr};

// ----- number -----

// number          = 1*DIGIT
//                    ; Unsigned 32-bit integer
//                    ; (0 <= n < 4,294,967,296)
pub fn number(i: &[u8]) -> IResult<&[u8], u32> {
    let (i, bytes) = digit1(i)?;
    match from_utf8(bytes).ok().and_then(|s| u32::from_str(s).ok()) {
        Some(v) => Ok((i, v)),
        None => Err(nom::Err::Error(nom::error::make_error(
            i,
            nom::error::ErrorKind::MapRes,
        ))),
    }
}

// same as `number` but 64-bit
pub fn number_64(i: &[u8]) -> IResult<&[u8], u64> {
    let (i, bytes) = digit1(i)?;
    match from_utf8(bytes).ok().and_then(|s| u64::from_str(s).ok()) {
        Some(v) => Ok((i, v)),
        None => Err(nom::Err::Error(nom::error::make_error(
            i,
            nom::error::ErrorKind::MapRes,
        ))),
    }
}

// seq-range       = seq-number ":" seq-number
//                    ; two seq-number values and all values between
//                    ; these two regardless of order.
//                    ; seq-number is a nz-number
pub fn sequence_range(i: &[u8]) -> IResult<&[u8], std::ops::RangeInclusive<u32>> {
    map(tuple((number, tag(":"), number)), |(s, _, e)| s..=e)(i)
}

// sequence-set    = (seq-number / seq-range) *("," sequence-set)
//                     ; set of seq-number values, regardless of order.
//                     ; Servers MAY coalesce overlaps and/or execute the
//                     ; sequence in any order.
pub fn sequence_set(i: &[u8]) -> IResult<&[u8], Vec<std::ops::RangeInclusive<u32>>> {
    separated_list1(tag(","), alt((sequence_range, map(number, |n| n..=n))))(i)
}

// ----- string -----

// string = quoted / literal
pub fn string(i: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((quoted, literal))(i)
}

// string bytes as utf8
pub fn string_utf8(i: &[u8]) -> IResult<&[u8], &str> {
    map_res(string, from_utf8)(i)
}

// quoted = DQUOTE *QUOTED-CHAR DQUOTE
pub fn quoted(i: &[u8]) -> IResult<&[u8], &[u8]> {
    delimited(
        char('"'),
        escaped(
            take_while1(|byte| is_text_char(byte) && !is_quoted_specials(byte)),
            '\\',
            one_of("\\\""),
        ),
        char('"'),
    )(i)
}

// quoted bytes as utf8
pub fn quoted_utf8(i: &[u8]) -> IResult<&[u8], &str> {
    map_res(quoted, from_utf8)(i)
}

// quoted-specials = DQUOTE / "\"
pub fn is_quoted_specials(c: u8) -> bool {
    c == b'"' || c == b'\\'
}

/// literal = "{" number "}" CRLF *CHAR8
///            ; Number represents the number of CHAR8s
pub fn literal(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let mut parser = tuple((tag(b"{"), number, tag(b"}"), tag("\r\n")));

    let (remaining, (_, count, _, _)) = parser(input)?;

    let (remaining, data) = take(count)(remaining)?;

    Ok((remaining, data))
}

// ----- astring ----- atom (roughly) or string

// astring = 1*ASTRING-CHAR / string
pub fn astring(i: &[u8]) -> IResult<&[u8], &[u8]> {
    alt((take_while1(is_astring_char), string))(i)
}

// astring bytes as utf8
pub fn astring_utf8(i: &[u8]) -> IResult<&[u8], &str> {
    map_res(astring, from_utf8)(i)
}

// ASTRING-CHAR = ATOM-CHAR / resp-specials
pub fn is_astring_char(c: u8) -> bool {
    is_atom_char(c) || is_resp_specials(c)
}

// ATOM-CHAR = <any CHAR except atom-specials>
pub fn is_atom_char(c: u8) -> bool {
    is_char(c) && !is_atom_specials(c)
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
pub fn atom(i: &[u8]) -> IResult<&[u8], &str> {
    map_res(take_while1(is_atom_char), from_utf8)(i)
}

// ----- nstring ----- nil or string

// nstring = string / nil
pub fn nstring(i: &[u8]) -> IResult<&[u8], Option<&[u8]>> {
    alt((map(nil, |_| None), map(string, Some)))(i)
}

// nstring bytes as utf8
pub fn nstring_utf8(i: &[u8]) -> IResult<&[u8], Option<&str>> {
    alt((map(nil, |_| None), map(string_utf8, Some)))(i)
}

// nil = "NIL"
pub fn nil(i: &[u8]) -> IResult<&[u8], &[u8]> {
    tag_no_case("NIL")(i)
}

// ----- text -----

// text = 1*TEXT-CHAR
pub fn text(i: &[u8]) -> IResult<&[u8], &str> {
    map_res(take_while(is_text_char), from_utf8)(i)
}

// TEXT-CHAR = <any CHAR except CR and LF>
pub fn is_text_char(c: u8) -> bool {
    is_char(c) && c != b'\r' && c != b'\n'
}

// CHAR = %x01-7F
//          ; any 7-bit US-ASCII character,
//          ;  excluding NUL
// From RFC5234
pub fn is_char(c: u8) -> bool {
    matches!(c, 0x01..=0x7F)
}

// ----- others -----

// list-wildcards = "%" / "*"
pub fn is_list_wildcards(c: u8) -> bool {
    c == b'%' || c == b'*'
}

pub fn paren_delimited<'a, F, O, E>(f: F) -> impl FnMut(&'a [u8]) -> IResult<&'a [u8], O, E>
where
    F: FnMut(&'a [u8]) -> IResult<&'a [u8], O, E>,
    E: nom::error::ParseError<&'a [u8]>,
{
    delimited(char('('), f, char(')'))
}

pub fn parenthesized_nonempty_list<'a, F, O, E>(
    f: F,
) -> impl FnMut(&'a [u8]) -> IResult<&'a [u8], Vec<O>, E>
where
    F: FnMut(&'a [u8]) -> IResult<&'a [u8], O, E>,
    E: nom::error::ParseError<&'a [u8]>,
{
    delimited(char('('), separated_list1(char(' '), f), char(')'))
}

pub fn parenthesized_list<'a, F, O, E>(f: F) -> impl FnMut(&'a [u8]) -> IResult<&'a [u8], Vec<O>, E>
where
    F: FnMut(&'a [u8]) -> IResult<&'a [u8], O, E>,
    E: nom::error::ParseError<&'a [u8]>,
{
    delimited(
        char('('),
        separated_list0(char(' '), f),
        preceded(
            opt(char(' ')), // Surgemail sometimes sends a space before the closing bracket.
            char(')'),
        ),
    )
}

pub fn opt_opt<'a, F, O, E>(mut f: F) -> impl FnMut(&'a [u8]) -> IResult<&'a [u8], Option<O>, E>
where
    F: FnMut(&'a [u8]) -> IResult<&'a [u8], Option<O>, E>,
{
    move |i: &[u8]| match f(i) {
        Ok((i, o)) => Ok((i, o)),
        Err(nom::Err::Error(_)) => Ok((i, None)),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    #[test]
    fn test_quoted() {
        let (rem, val) = quoted(br#""Hello"???"#).unwrap();
        assert_eq!(rem, b"???");
        assert_eq!(val, b"Hello");

        // Allowed escapes...
        assert_eq!(
            quoted(br#""Hello \" "???"#),
            Ok((&b"???"[..], &br#"Hello \" "#[..]))
        );
        assert_eq!(
            quoted(br#""Hello \\ "???"#),
            Ok((&b"???"[..], &br#"Hello \\ "#[..]))
        );

        // Not allowed escapes...
        assert!(quoted(br#""Hello \a "???"#).is_err());
        assert!(quoted(br#""Hello \z "???"#).is_err());
        assert!(quoted(br#""Hello \? "???"#).is_err());

        let (rem, val) = quoted(br#""Hello \"World\""???"#).unwrap();
        assert_eq!(rem, br#"???"#);
        // Should it be this (Hello \"World\") ...
        assert_eq!(val, br#"Hello \"World\""#);
        // ... or this (Hello "World")?
        //assert_eq!(val, br#"Hello "World""#); // fails

        // Test Incomplete
        assert_matches!(quoted(br#""#), Err(nom::Err::Incomplete(_)));
        assert_matches!(quoted(br#""\"#), Err(nom::Err::Incomplete(_)));
        assert_matches!(quoted(br#""Hello "#), Err(nom::Err::Incomplete(_)));

        // Test Error
        assert_matches!(quoted(br"\"), Err(nom::Err::Error(_)));
    }

    #[test]
    fn test_string_literal() {
        match string(b"{3}\r\nXYZ") {
            Ok((_, value)) => {
                assert_eq!(value, b"XYZ");
            }
            rsp => panic!("unexpected response {rsp:?}"),
        }
    }

    #[test]
    fn test_string_literal_containing_null() {
        match string(b"{5}\r\nX\0Y\0Z") {
            Ok((_, value)) => {
                assert_eq!(value, b"X\0Y\0Z");
            }
            rsp => panic!("unexpected response {rsp:?}"),
        }
    }

    #[test]
    fn test_astring() {
        match astring(b"text ") {
            Ok((_, value)) => {
                assert_eq!(value, b"text");
            }
            rsp => panic!("unexpected response {rsp:?}"),
        }
    }

    #[test]
    fn test_sequence_range() {
        match sequence_range(b"23:28 ") {
            Ok((_, value)) => {
                assert_eq!(*value.start(), 23);
                assert_eq!(*value.end(), 28);
                assert_eq!(value.collect::<Vec<u32>>(), vec![23, 24, 25, 26, 27, 28]);
            }
            rsp => panic!("Unexpected response {rsp:?}"),
        }
    }

    #[test]
    fn test_sequence_set() {
        match sequence_set(b"1,2:8,10,15:30 ") {
            Ok((_, value)) => {
                assert_eq!(value.len(), 4);
                let v = &value[0];
                assert_eq!(*v.start(), 1);
                assert_eq!(*v.end(), 1);
                let v = &value[1];
                assert_eq!(*v.start(), 2);
                assert_eq!(*v.end(), 8);
                let v = &value[2];
                assert_eq!(*v.start(), 10);
                assert_eq!(*v.end(), 10);
                let v = &value[3];
                assert_eq!(*v.start(), 15);
                assert_eq!(*v.end(), 30);
            }
            rsp => panic!("Unexpected response {rsp:?}"),
        }
    }
}
