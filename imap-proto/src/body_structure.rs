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
    delimited!(
        char!('('),
        map!(separated_nonempty_list!(opt!(tag!(" ")), body_param), |params| Some(params)),
        char!(')')
    )
));

named!(body_disposition<Option<BodyDisposition>>, alt!(
    map!(tag_s!("NIL"), |_| None) |
    delimited!(
        char!('('),
        do_parse!(
            typ: string_utf8 >>
            tag_s!(" ") >>
            params: body_params >>
            (Some(BodyDisposition { disposition_type: typ, params }))
        ),
        char!(')')
    )
));

named!(body_extension<BodyExtension>, alt!(
    map!(number, |n| BodyExtension::Num(n)) |
    map!(nstring_utf8, |s| BodyExtension::Str(s)) |
    delimited!(
        char!('('),
        map!(separated_nonempty_list!(opt!(tag!(" ")), body_extension), |exts| BodyExtension::List(exts)),
        char!(')')
    )
));

named!(body_lang<Option<Vec<&str>>>, alt!(
    map!(nstring_utf8, |v| v.map(|s| vec![s])) |
    delimited!(
        char!('('),
        map!(separated_nonempty_list!(opt!(tag!(" ")), string_utf8), |langs| Some(langs)),
        char!(')')
    )
));

named!(body_field_extension_opt<Option<BodyExtension>>, opt!(complete!(do_parse!(
    tag_s!(" ") >>
    ext: body_extension >>
    (ext)
))));

named!(body_type_basic<BodyStructure>, do_parse!(
    media_type: string_utf8 >>
    tag_s!(" ") >>
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
    md5: nstring_utf8 >>
    tag_s!(" ") >>
    disposition: body_disposition >>
    tag_s!(" ") >>
    lang: body_lang >>
    tag_s!(" ") >>
    loc: nstring_utf8 >>
    extensions: body_field_extension_opt >>
    (BodyStructure::Basic(BodyStructureBasic {
        media_type,
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
        extensions
    }))
));

named!(body_type_text<BodyStructure>, do_parse!(
    tag_s!("\"TEXT\"") >>
    tag_s!(" ") >>
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
    extensions: body_field_extension_opt >>
    (BodyStructure::Text(BodyStructureText {
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
    }))
));

named!(body_type<BodyStructure>, alt!(
    body_type_text | body_type_basic
));

named!(pub msg_att_body_structure<AttributeValue>, do_parse!(
    tag_s!("BODYSTRUCTURE (") >>
    body: body_type >>
    tag_s!(")") >>
    (AttributeValue::BodyStructure(Box::new(body)))
));

#[cfg(test)]
mod tests {
    use super::body_type;
    use types::*;

    #[test]
    fn test_body_structure_text() {
        const RESPONSE: &[u8] = br#""TEXT" "PLAIN" ("CHARSET" "iso-8859-1") NIL NIL "QUOTED-PRINTABLE" 1315 42 NIL NIL NIL NIL"#;
        match body_type(RESPONSE) {
            Ok((_, BodyStructure::Text(text))) => {
                // assert_eq!(text, BodyStructureText {})
            },
            rsp @ _ => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_body_structure_basic() {
        const RESPONSE: &[u8] = br#""APPLICATION" "PDF" ("NAME" "pages.pdf") NIL NIL "BASE64" 38838 NIL ("attachment" ("FILENAME" "pages.pdf")) NIL NIL"#;
        match body_type(RESPONSE) {
            Ok((_, BodyStructure::Basic(basic))) => {
                // assert_eq!(text, BodyStructureText {})
            },
            rsp @ _ => panic!("unexpected response {:?}", rsp),
        }
    }
}
