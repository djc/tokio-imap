//!
//! https://tools.ietf.org/html/rfc4315
//!
//! The IMAP UIDPLUS Extension
//!

use nom::{
    branch::alt,
    bytes::streaming::{tag, tag_no_case},
    combinator::map,
    multi::separated_list1,
    sequence::{preceded, tuple},
    IResult,
};

use crate::parser::core::number;
use crate::types::*;

/// Extends resp-text-code as follows:
///
///     resp-text-code =/ resp-code-apnd
///     resp-code-apnd = "APPENDUID" SP nz-number SP append-uid
///     append-uid      =/ uid-set
///                       ; only permitted if client uses [MULTIAPPEND]
///                       ; to append multiple messages.
///
/// [RFC4315 - 3 Additional Response Codes](https://tools.ietf.org/html/rfc4315#section-3)
pub(crate) fn resp_text_code_append_uid(i: &[u8]) -> IResult<&[u8], ResponseCode> {
    map(
        preceded(
            tag_no_case(b"APPENDUID "),
            tuple((number, tag(" "), uid_set)),
        ),
        |(fst, _, snd)| ResponseCode::AppendUid(fst, snd),
    )(i)
}

/// Extends resp-text-code as follows:
///
///     resp-text-code =/ resp-code-copy
///     resp-code-copy = "COPYUID" SP nz-number SP uid-set
///
/// [RFC4315 - 3 Additional Response Codes](https://tools.ietf.org/html/rfc4315#section-3)
pub(crate) fn resp_text_code_copy_uid(i: &[u8]) -> IResult<&[u8], ResponseCode> {
    map(
        preceded(
            tag_no_case(b"COPYUID "),
            tuple((number, tag(" "), uid_set, tag(" "), uid_set)),
        ),
        |(fst, _, snd, _, trd)| ResponseCode::CopyUid(fst, snd, trd),
    )(i)
}

/// Extends resp-text-code as follows:
///
///     resp-text-code =/ "UIDNOTSTICKY"
///
/// [RFC4315 - 3 Additional Response Codes](https://tools.ietf.org/html/rfc4315#section-3)
pub(crate) fn resp_text_code_uid_not_sticky(i: &[u8]) -> IResult<&[u8], ResponseCode> {
    map(tag_no_case(b"UIDNOTSTICKY"), |_| ResponseCode::UidNotSticky)(i)
}

/// Parses the uid-set nonterminal:
///
///     uid-set = (uniqueid / uid-range) *("," uid-set)
///
/// [RFC4315 - 4 Formal Syntax](https://tools.ietf.org/html/rfc4315#section-4)
fn uid_set(i: &[u8]) -> IResult<&[u8], Vec<UidSetMember>> {
    separated_list1(tag(","), alt((uid_range, map(number, From::from))))(i)
}

/// Parses the uid-set nonterminal:
///
///    uid-range = (uniqueid ":" uniqueid)
///                ; two uniqueid values and all values
///                ; between these two regards of order.
///                ; Example: 2:4 and 4:2 are equivalent.
///
/// [RFC4315 - 4 Formal Syntax](https://tools.ietf.org/html/rfc4315#section-4)
fn uid_range(i: &[u8]) -> IResult<&[u8], UidSetMember> {
    map(
        nom::sequence::separated_pair(number, tag(":"), number),
        |(fst, snd)| if fst <= snd { fst..=snd } else { snd..=fst }.into(),
    )(i)
}
