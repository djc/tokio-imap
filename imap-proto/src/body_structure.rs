use core::*;
use types::*;

use parser::envelope;

use std::str;

named!(body_lang<Option<Vec<&str>>>, alt!(
    map!(nstring_utf8, |v| v.map(|s| vec![s])) |
    delimited!(
        char!('('),
        map!(separated_nonempty_list!(opt!(space), string_utf8), Option::from),
        char!(')')
    )
));

named!(body_param<Option<Vec<BodyParam>>>, alt!(
    map!(nil, |_| None) |
    delimited!(
        char!('('),
        map!(separated_nonempty_list!(opt!(space), do_parse!(
            key: string_utf8 >>
            space >>
            val: string_utf8 >>
            (BodyParam { key, val })
        )), |params| Some(params)),
        char!(')')
    )
));

named!(body_extension<BodyExtension>, alt!(
    dbg!(map!(number, |n| BodyExtension::Num(n))) |
    map!(nstring_utf8, |s| BodyExtension::Str(s)) |
    delimited!(
        char!('('),
        map!(
            separated_nonempty_list!(opt!(space), body_extension),
            |exts| BodyExtension::List(exts)
        ),
        char!(')')
    )
));

named!(body_disposition<Option<BodyDisposition>>, alt!(
    map!(nil, |_| None) |
    delimited!(
        char!('('),
        do_parse!(
            typ: string_utf8 >>
            space >>
            params: body_param >>
            (Some(BodyDisposition { disposition_type: typ, params }))
        ),
        char!(')')
    )
));

named!(body_type_basic<BodyStructure>, do_parse!(
    media_type: string_utf8 >>
    space >>
    media_subtype: string_utf8 >>
    space >>
    params: body_param >>
    space >>
    id: nstring_utf8 >>
    space >>
    description: nstring_utf8 >>
    space >>
    encoding: string_utf8 >>
    space >>
    octets: number >>
    md5: opt_opt!(preceded!(space, nstring_utf8)) >>
    disposition: opt_opt!(preceded!(space, body_disposition)) >>
    lang: opt_opt!(preceded!(space, body_lang)) >>
    loc: opt_opt!(preceded!(space, nstring_utf8)) >>
    extensions: opt!(preceded!(space, body_extension)) >>
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
    space >>
    media_subtype: string_utf8 >>
    space >>
    params: body_param >>
    space >>
    id: nstring_utf8 >>
    space >>
    description: nstring_utf8 >>
    space >>
    encoding: string_utf8 >>
    space >>
    octets: number >>
    space >>
    lines: number >>
    md5: opt_opt!(preceded!(space, nstring_utf8)) >>
    disposition: opt_opt!(preceded!(space, body_disposition)) >>
    lang: opt_opt!(preceded!(space, body_lang)) >>
    loc: opt_opt!(preceded!(space, nstring_utf8)) >>
    extensions: opt!(preceded!(space, body_extension)) >>
    (BodyStructure::Text(BodyStructureText {
        media_subtype,
        params,
        id,
        description,
        encoding,
        octets,
        lines,
        md5,
        disposition,
        lang,
        loc,
        extensions
    }))
));

named!(body_type_message<BodyStructure>, do_parse!(
    tag_s!("\"MESSAGE\" \"RFC822\"") >>
    space >>
    params: body_param >>
    space >>
    id: nstring_utf8 >>
    space >>
    description: nstring_utf8 >>
    space >>
    encoding: string_utf8 >>
    space >>
    octets: number >>
    space >>
    envelope: envelope >>
    space >>
    body: body >>
    space >>
    lines: number >>
    (BodyStructure::Message(BodyStructureMessage {
        params,
        id,
        description,
        encoding,
        octets,
        envelope: Box::new(envelope),
        body: Box::new(body),
        lines
    }))
));


// body-fld-param [SP body-fld-dsp [SP body-fld-lang
//   [SP body-fld-loc *(SP body-extension)]]]
//   ; MUST NOT be returned on non-extensible
//   ; "BODY" fetch
named!(body_type_multipart<BodyStructure>, do_parse!(
    bodies: many1!(body) >>
    space >>
    media_subtype: string_utf8 >>
    params: opt_opt!(preceded!(space, body_param)) >>
    disposition: opt_opt!(preceded!(space, body_disposition)) >>
    lang: opt_opt!(preceded!(space, body_lang)) >>
    loc: opt_opt!(preceded!(space, nstring_utf8)) >>
    extensions: opt!(preceded!(space, body_extension)) >>
    (BodyStructure::Multipart(BodyStructureMultipart {
        bodies,
        media_subtype,
        params,
        disposition,
        lang,
        loc,
        extensions
    }))
));

named!(body<BodyStructure>, delimited!(
    char!('('),
    alt!(
        body_type_text | body_type_basic |
        body_type_message | body_type_multipart
    ),
    char!(')')
));

named!(pub msg_att_body_structure<AttributeValue>, do_parse!(
    tag_s!("BODYSTRUCTURE ") >>
    body: body >>
    (AttributeValue::BodyStructure(Box::new(body)))
));

#[cfg(test)]
mod tests {
    use super::*;

    const EMPTY: &[u8] = &[];

    #[test]
    fn test_body_param_nil() {
        assert_eq!(
            body_param(br#"NIL"#).unwrap(),
            (EMPTY, None)
        );
    }

    #[test]
    fn test_body_param() {
        assert_eq!(
            body_param(br#"("foo" "bar")"#).unwrap(),
            (EMPTY, Some(vec![BodyParam { key: "foo", val: "bar" }]))
        );
    }

    #[test]
    fn test_body_lang_one() {
        assert_eq!(
            body_lang(br#""bob""#).unwrap(),
            (EMPTY, Some(vec!["bob"]))
        )
    }

    #[test]
    fn test_body_lang_list() {
        assert_eq!(
            body_lang(br#"("one" "two")"#).unwrap(),
            (EMPTY, Some(vec!["one", "two"]))
        )
    }

    #[test]
    fn test_body_lang_nil() {
        assert_eq!(
            body_lang(br#"NIL"#).unwrap(),
            (EMPTY, None)
        )
    }

    #[test]
    fn test_body_extension_str() {
        assert_eq!(
            body_extension(br#""blah""#).unwrap(),
            (EMPTY, BodyExtension::Str(Some("blah")))
        )
    }

    #[test]
    fn test_body_extension_str_nil() {
        assert_eq!(
            body_extension(br#"NIL"#).unwrap(),
            (EMPTY, BodyExtension::Str(None))
        )
    }

    #[test]
    fn test_body_extension_list() {
        assert_eq!(
            body_extension(br#"("hello")"#).unwrap(),
            (EMPTY, BodyExtension::List(vec![BodyExtension::Str(Some("hello"))]))
        )
    }

    #[test]
    fn test_body_extension_list_num() {
        assert_eq!(
            body_extension(br#"(1337)"#).unwrap(),
            (EMPTY, BodyExtension::List(vec![BodyExtension::Num(1337)]))
        )
    }

    #[test]
    fn test_body_disposition_nil() {
        assert_eq!(
            body_disposition(br#"NIL"#).unwrap(),
            (EMPTY, None)
        )
    }

    #[test]
    fn test_body_disposition_simple() {
        assert_eq!(
            body_disposition(br#"("attachment" ("FILENAME" "pages.pdf"))"#).unwrap(),
            (EMPTY, Some(BodyDisposition {
                disposition_type: "attachment",
                params: Some(vec![BodyParam { key: "FILENAME", val: "pages.pdf" }])
            }))
        )
    }

    #[test]
    fn test_body_structure_text() {
        const BODY: &[u8] = br#"("TEXT" "PLAIN" ("CHARSET" "US-ASCII") NIL NIL "7BIT" 2279 48)"#;
        match body(BODY) {
            Ok((_, BodyStructure::Text(text))) => {
                // assert_eq!(text, BodyStructureText {})
            },
            rsp @ _ => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_body_structure_text_with_ext() {
        const BODY: &[u8] = br#"("TEXT" "PLAIN" ("CHARSET" "iso-8859-1") NIL NIL "QUOTED-PRINTABLE" 1315 42 NIL NIL NIL NIL)"#;
        match body(BODY) {
            Ok((_, BodyStructure::Text(text))) => {
                // assert_eq!(text, BodyStructureText {})
            },
            rsp @ _ => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_body_structure_basic() {
        const BODY: &[u8] = br#"("APPLICATION" "PDF" ("NAME" "pages.pdf") NIL NIL "BASE64" 38838 NIL ("attachment" ("FILENAME" "pages.pdf")) NIL NIL)"#;
        match body(BODY) {
            Ok((_, BodyStructure::Basic(basic))) => {
                // assert_eq!(text, BodyStructureText {})
            },
            rsp @ _ => panic!("unexpected response {:?}", rsp),
        }
    }
}
