use core::*;
use types::*;

named!(pub section_part<Vec<u32>>, do_parse!(
    part: number >>
    rest: many0!(do_parse!(
        tag_s!(".") >>
        part: number >>
        (part)
    ))  >> ({
        let mut res = vec![part];
        res.extend(rest);
        res
    })
));

named!(pub section_msgtext<MessageSection>, map!(
    alt!(tag_s!("HEADER") | tag_s!("TEXT")),
    |s| match s {
        b"HEADER" => MessageSection::Header,
        b"TEXT" => MessageSection::Text,
        _ => panic!("cannot happen"),
    }
));

named!(pub section_text<MessageSection>, alt!(
    section_msgtext |
    do_parse!(tag_s!("MIME") >> (MessageSection::Mime))
));

named!(pub section_spec<SectionPath>, alt!(
    map!(section_msgtext, |val| SectionPath::Full(val)) |
    do_parse!(
        part: section_part >>
        text: opt!(do_parse!(
            tag_s!(".") >>
            text: section_text >>
            (text)
        )) >>
        (SectionPath::Part(part, text))
    )
));

named!(pub section<Option<SectionPath>>, do_parse!(
    tag_s!("[") >>
    spec: opt!(section_spec) >>
    tag_s!("]") >>
    (spec)
));

named!(pub msg_att_body_section<AttributeValue>, do_parse!(
    tag_s!("BODY") >>
    section: section >>
    index: opt!(do_parse!(
        tag_s!("<") >>
        num: number >>
        tag_s!(">") >>
        (num)
    )) >>
    tag_s!(" ") >>
    data: nstring >>
    (AttributeValue::BodySection { section, index, data })
));
