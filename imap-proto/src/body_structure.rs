use core::*;
use types::*;

use std::str;

named!(body_param<BodyParam>, do_parse!(
    key: string_utf8 >>
    tag_s!(" ") >>
    val: string_utf8 >>
    (BodyParam { key, val })
));

named!(body_params<Option<Vec<BodyParam>>>, alt!(
    map!(tag_s!("NIL"), |_| None) |
    do_parse!(
        tag_s!("(") >>
        param0: body_param >>
        params: many0!(do_parse!(
            tag_s!(" ") >>
            param: body_param >>
            (param)
        )) >>
        tag_s!(")") >>
        ({
            let mut res = vec![param0];
            res.extend(params);
            Some(res)
        })
    )
));

named!(body_disposition<Option<BodyDisposition>>, alt!(
    map!(tag_s!("NIL"), |_| None) |
    do_parse!(
        tag_s!("(") >>
        typ: string_utf8 >>
        tag_s!(" ") >>
        params: body_params >>
        tag_s!(")") >>
        (Some(BodyDisposition { disposition_type: typ, params }))
    )
));

named!(body_extension<BodyExtension>, alt!(
    map!(number, |n| BodyExtension::Num(n)) |
    map!(nstring_utf8, |s| BodyExtension::Str(s)) |
    do_parse!(
        tag_s!("(") >>
        ext0: body_extension >>
        rest: many0!(do_parse!(
            tag_s!(" ") >>
            ext: body_extension >>
            (ext)
        )) >>
        tag_s!(")") >>
        ({
            let mut exts = vec![ext0];
            exts.extend(rest);
            BodyExtension::List(exts)
        })
    )
));

named!(body_lang<Option<Vec<&str>>>, alt!(
    map!(nstring_utf8, |v| v.map(|s| vec![s])) |
    do_parse!(
        tag_s!("(") >>
        lang0: string_utf8 >>
        rest: many0!(do_parse!(
            tag_s!(" ") >>
            lang: string_utf8 >>
            (lang)
        )) >>
        tag_s!(")") >>
        ({
            let mut langs = vec![lang0];
            langs.extend(rest);
            Some(langs)
        })
    )
));

named!(body_type_text<BodyStructure>, do_parse!(
    tag_s!("\"TEXT\" ") >>
    media_subtype: string_utf8 >>
    tag_s!(" ") >>
    params: body_params >>
    tag_s!(" ") >>
    id: nstring_utf8 >>
    tag_s!(" ") >>
    description: nstring_utf8 >>
    tag_s!(" ") >>
    encoding: string_utf8 >>
    tag_s!(" ") >>
    octets: number >>
    tag_s!(" ") >>
    lines: number >>
    tag_s!(" ") >>
    md5: nstring_utf8 >>
    tag_s!(" ") >>
    disposition: body_disposition >>
    tag_s!(" ") >>
    lang: body_lang >>
    tag_s!(" ") >>
    loc: nstring_utf8 >>
    extensions: opt!(do_parse!(
        tag_s!(" ") >>
        ext: body_extension >>
        (ext)
    )) >>
    (BodyStructure::Text {
        media_subtype,
        params,
        id,
        description,
        encoding,
        octets,
        md5,
        disposition,
        lang,
        loc,
        lines,
        extensions
    })
));

named!(body_type<BodyStructure>, alt!(
    body_type_text
));

named!(pub msg_att_body_structure<AttributeValue>, do_parse!(
    tag_s!("BODYSTRUCTURE (") >>
    body: body_type >>
    tag_s!(")") >>
    (AttributeValue::BodyStructure(Box::new(body)))
));
