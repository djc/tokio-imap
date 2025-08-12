use nom::{
    branch::alt,
    bytes::streaming::{tag, tag_no_case},
    character::streaming::char,
    combinator::{map, opt},
    multi::many0,
    sequence::{delimited, preceded, tuple},
    IResult,
};
use std::borrow::Cow;

use crate::{parser::core::*, types::*};

pub fn section_part(i: &[u8]) -> IResult<&[u8], Vec<u32>> {
    let (i, (part, mut rest)) = tuple((number, many0(preceded(char('.'), number))))(i)?;
    rest.insert(0, part);
    Ok((i, rest))
}

pub fn section_msgtext(i: &[u8]) -> IResult<&[u8], MessageSection> {
    alt((
        map(
            tuple((
                tag_no_case("HEADER.FIELDS"),
                opt(tag_no_case(".NOT")),
                tag(" "),
                parenthesized_list(astring),
            )),
            |_| MessageSection::Header,
        ),
        map(tag_no_case("HEADER"), |_| MessageSection::Header),
        map(tag_no_case("TEXT"), |_| MessageSection::Text),
    ))(i)
}

pub fn section_text(i: &[u8]) -> IResult<&[u8], MessageSection> {
    alt((
        section_msgtext,
        map(tag_no_case("MIME"), |_| MessageSection::Mime),
    ))(i)
}

pub fn section_spec(i: &[u8]) -> IResult<&[u8], SectionPath> {
    alt((
        map(section_msgtext, SectionPath::Full),
        map(
            tuple((section_part, opt(preceded(char('.'), section_text)))),
            |(part, text)| SectionPath::Part(part, text),
        ),
    ))(i)
}

pub fn section(i: &[u8]) -> IResult<&[u8], Option<SectionPath>> {
    delimited(char('['), opt(section_spec), char(']'))(i)
}

pub fn msg_att_body_section(i: &[u8]) -> IResult<&[u8], AttributeValue<'_>> {
    map(
        tuple((
            tag_no_case("BODY"),
            section,
            opt(delimited(char('<'), number, char('>'))),
            tag(" "),
            nstring,
        )),
        |(_, section, index, _, data)| AttributeValue::BodySection {
            section,
            index,
            data: data.map(Cow::Borrowed),
        },
    )(i)
}
