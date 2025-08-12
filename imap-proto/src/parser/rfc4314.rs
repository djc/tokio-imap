//!
//! Current
//! https://tools.ietf.org/html/rfc4314
//!
//! Original
//! https://tools.ietf.org/html/rfc2086
//!
//! The IMAP ACL Extension
//!

use std::borrow::Cow;

use nom::{
    bytes::streaming::tag_no_case,
    character::complete::{space0, space1},
    combinator::map,
    multi::separated_list0,
    sequence::{preceded, separated_pair, tuple},
    IResult,
};

use crate::parser::core::astring_utf8;
use crate::parser::rfc3501::mailbox;
use crate::types::*;

/// 3.6. ACL Response
/// ```ignore
/// acl_response  ::= "ACL" SP mailbox SP acl_list
/// ```
pub(crate) fn acl(i: &[u8]) -> IResult<&[u8], Response<'_>> {
    let (rest, (_, _, mailbox, acls)) = tuple((
        tag_no_case("ACL"),
        space1,
        map(mailbox, Cow::Borrowed),
        acl_list,
    ))(i)?;

    Ok((rest, Response::Acl(Acl { mailbox, acls })))
}

/// ```ignore
/// acl_list  ::= *(SP acl_entry)
/// ```
fn acl_list(i: &[u8]) -> IResult<&[u8], Vec<AclEntry<'_>>> {
    preceded(space0, separated_list0(space1, acl_entry))(i)
}

/// ```ignore
/// acl_entry ::= SP identifier SP rights
/// ```
fn acl_entry(i: &[u8]) -> IResult<&[u8], AclEntry<'_>> {
    let (rest, (identifier, rights)) = separated_pair(
        map(astring_utf8, Cow::Borrowed),
        space1,
        map(astring_utf8, map_text_to_rights),
    )(i)?;

    Ok((rest, AclEntry { identifier, rights }))
}

/// 3.7. LISTRIGHTS Response
/// ```ignore
/// list_rights_response  ::= "LISTRIGHTS" SP mailbox SP identifier SP required_rights *(SP optional_rights)
/// ```
pub(crate) fn list_rights(i: &[u8]) -> IResult<&[u8], Response<'_>> {
    let (rest, (_, _, mailbox, _, identifier, _, required, optional)) = tuple((
        tag_no_case("LISTRIGHTS"),
        space1,
        map(mailbox, Cow::Borrowed),
        space1,
        map(astring_utf8, Cow::Borrowed),
        space1,
        map(astring_utf8, map_text_to_rights),
        list_rights_optional,
    ))(i)?;

    Ok((
        rest,
        Response::ListRights(ListRights {
            mailbox,
            identifier,
            required,
            optional,
        }),
    ))
}

fn list_rights_optional(i: &[u8]) -> IResult<&[u8], Vec<AclRight>> {
    let (rest, items) = preceded(space0, separated_list0(space1, astring_utf8))(i)?;

    Ok((
        rest,
        items
            .into_iter()
            .flat_map(|s| s.chars().map(|c| c.into()))
            .collect(),
    ))
}

/// 3.7. MYRIGHTS Response
/// ```ignore
/// my_rights_response  ::= "MYRIGHTS" SP mailbox SP rights
/// ```
pub(crate) fn my_rights(i: &[u8]) -> IResult<&[u8], Response<'_>> {
    let (rest, (_, _, mailbox, _, rights)) = tuple((
        tag_no_case("MYRIGHTS"),
        space1,
        map(mailbox, Cow::Borrowed),
        space1,
        map(astring_utf8, map_text_to_rights),
    ))(i)?;

    Ok((rest, Response::MyRights(MyRights { mailbox, rights })))
}

/// helper routine to map a string to a vec of AclRights
fn map_text_to_rights(i: &str) -> Vec<AclRight> {
    i.chars().map(|c| c.into()).collect()
}
