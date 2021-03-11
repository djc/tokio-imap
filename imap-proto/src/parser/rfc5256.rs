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

/// BASE.7.2.SORT. SORT Response
///
///    Data:       zero or more numbers
///
///       The SORT response occurs as a result of a SORT or UID SORT
///       command.  The number(s) refer to those messages that match the
///       search criteria.  For SORT, these are message sequence numbers;
///       for UID SORT, these are unique identifiers.  Each number is
///       delimited by a space.
///
///    Example:    S: * SORT 2 3 6
///
/// [RFC5256 - 4 Additional Responses](https://tools.ietf.org/html/rfc5256#section-4)
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
