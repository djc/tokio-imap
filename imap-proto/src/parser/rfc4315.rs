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

use either::Either;

use crate::parser::core::number;
use crate::types::*;

pub(crate) fn resp_text_code_append_uid(i: &[u8]) -> IResult<&[u8], ResponseCode> {
    map(
        preceded(
            tag_no_case(b"APPENDUID "),
            tuple((number, tag(" "), uid_set)),
        ),
        |(fst, _, snd)| ResponseCode::AppendUid(fst, snd),
    )(i)
}

pub(crate) fn resp_text_code_copy_uid(i: &[u8]) -> IResult<&[u8], ResponseCode> {
    map(
        preceded(
            tag_no_case(b"COPYUID "),
            tuple((number, tag(" "), uid_set, tag(" "), uid_set)),
        ),
        |(fst, _, snd, _, trd)| ResponseCode::CopyUid(fst, snd, trd),
    )(i)
}

pub(crate) fn resp_text_code_uid_not_sticky(i: &[u8]) -> IResult<&[u8], ResponseCode> {
    map(tag_no_case(b"UIDNOTSTICKY"), |_| ResponseCode::UidNotSticky)(i)
}

fn uid_set(i: &[u8]) -> IResult<&[u8], Vec<Either<std::ops::RangeInclusive<u32>, u32>>> {
    separated_list1(tag(","), alt((uid_range, map(number, Either::Right))))(i)
}

fn uid_range(i: &[u8]) -> IResult<&[u8], Either<std::ops::RangeInclusive<u32>, u32>> {
    map(
        nom::sequence::separated_pair(number, tag(":"), number),
        |(fst, snd)| Either::Left(if fst <= snd { fst..=snd } else { snd..=fst }),
    )(i)
}
