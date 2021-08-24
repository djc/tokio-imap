use nom::{
    branch::alt,
    bytes::streaming::{tag, tag_no_case, take_while},
    combinator::{map, map_res, recognize},
    sequence::pair,
    IResult,
};
use std::{borrow::Cow, str::from_utf8};

use crate::{parser::core::*, types::*};

fn no_inferiors(i: &[u8]) -> IResult<&[u8], NameAttribute> {
    map(tag_no_case(b"\\Noinferiors"), |_s| {
        NameAttribute::NoInferiors
    })(i)
}

fn no_select(i: &[u8]) -> IResult<&[u8], NameAttribute> {
    map(tag_no_case(b"\\Noselect"), |_s| NameAttribute::NoSelect)(i)
}

fn marked(i: &[u8]) -> IResult<&[u8], NameAttribute> {
    map(tag_no_case(b"\\Marked"), |_s| NameAttribute::Marked)(i)
}

fn unmarked(i: &[u8]) -> IResult<&[u8], NameAttribute> {
    map(tag_no_case(b"\\Unmarked"), |_s| NameAttribute::Unmarked)(i)
}

fn extension_str(i: &[u8]) -> IResult<&[u8], &str> {
    map_res(
        recognize(pair(tag(b"\\"), take_while(is_atom_char))),
        from_utf8,
    )(i)
}

fn extension(i: &[u8]) -> IResult<&[u8], NameAttribute> {
    map(extension_str, |s| {
        NameAttribute::Extension(Cow::Borrowed(s))
    })(i)
}

fn name_attribute(i: &[u8]) -> IResult<&[u8], NameAttribute> {
    alt((no_inferiors, no_select, marked, unmarked, extension))(i)
}

pub(crate) fn name_attributes(i: &[u8]) -> IResult<&[u8], Vec<NameAttribute>> {
    parenthesized_list(name_attribute)(i)
}
