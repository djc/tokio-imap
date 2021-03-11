//!
//! https://tools.ietf.org/html/rfc5256
//!
//! SORT extension
//!

use nom::{
    bytes::streaming::{tag, tag_no_case},
    combinator::{map, opt},
    multi::many0,
    sequence::{preceded, terminated},
    IResult,
};

use crate::{parser::core::number, types::MailboxDatum};

pub(crate) fn mailbox_data_sort(i: &[u8]) -> IResult<&[u8], MailboxDatum> {
    map(
        // Technically, trailing whitespace is not allowed here, but multiple
        // email servers in the wild seem to have it anyway (see #34, #108).
        terminated(
            preceded(tag_no_case(b"SORT"), many0(preceded(tag(" "), number))),
            opt(tag(" ")),
        ),
        MailboxDatum::Sort,
    )(i)
}
