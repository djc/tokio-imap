// rustfmt doesn't do a very good job on nom parser invocations.
#![cfg_attr(rustfmt, rustfmt_skip)]

use core::*;
use types::*;

use parser::envelope;

named!(body_lang<Option<Vec<&str>>>, alt!(
    map!(nstring_utf8, |v| v.map(|s| vec![s])) |
    map!(paren_list!(string_utf8), Option::from))
);

named!(body_param<Option<Vec<BodyParam>>>, alt!(
    map!(nil, |_| None) |
    map!(paren_list!(do_parse!(
        key: string_utf8 >>
        space >>
        val: string_utf8 >>
        (BodyParam { key, val })
    )), Option::from)
));

named!(body_extension<BodyExtension>, alt!(
    map!(number, |n| BodyExtension::Num(n)) |
    map!(nstring_utf8, |s| BodyExtension::Str(s)) |
    map!(paren_list!(body_extension), |ext| BodyExtension::List(ext))
));

named!(body_disposition<Option<BodyDisposition>>, alt!(
    map!(nil, |_| None) |
    paren_delimited!(do_parse!(
        typ: string_utf8 >>
        space >>
        param: body_param >>
        (Some(BodyDisposition {
            disposition_type: typ,
            param
        }))
    ))
));

named!(body_type_basic<BodyStructure>, do_parse!(
    media_type: string_utf8 >>
    space >>
    media_subtype: string_utf8 >>
    space >>
    param: body_param >>
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
    location: opt_opt!(preceded!(space, nstring_utf8)) >>
    extension: opt!(preceded!(space, body_extension)) >>
    (BodyStructure::Basic(BodyStructureBasic {
        media_type,
        media_subtype,
        param,
        id,
        description,
        encoding,
        octets,
        md5,
        disposition,
        lang,
        location,
        extension
    }))
));

named!(body_type_text<BodyStructure>, do_parse!(
    tag_no_case_s!("\"TEXT\"") >>
    space >>
    media_subtype: string_utf8 >>
    space >>
    param: body_param >>
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
    location: opt_opt!(preceded!(space, nstring_utf8)) >>
    extension: opt!(preceded!(space, body_extension)) >>
    (BodyStructure::Text(BodyStructureText {
        media_subtype,
        param,
        id,
        description,
        encoding,
        octets,
        lines,
        md5,
        disposition,
        lang,
        location,
        extension
    }))
));

named!(body_type_message<BodyStructure>, do_parse!(
    tag_no_case_s!("\"MESSAGE\" \"RFC822\"") >>
    space >>
    param: body_param >>
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
    md5: opt_opt!(preceded!(space, nstring_utf8)) >>
    disposition: opt_opt!(preceded!(space, body_disposition)) >>
    lang: opt_opt!(preceded!(space, body_lang)) >>
    location: opt_opt!(preceded!(space, nstring_utf8)) >>
    extension: opt!(preceded!(space, body_extension)) >>
    (BodyStructure::Message(BodyStructureMessage {
        param,
        id,
        description,
        encoding,
        octets,
        envelope: Box::new(envelope),
        body: Box::new(body),
        lines,
        md5,
        disposition,
        lang,
        location,
        extension,
    }))
));

named!(body_type_multipart<BodyStructure>, do_parse!(
    bodies: many1!(body) >>
    space >>
    media_subtype: string_utf8 >>
    param: opt_opt!(preceded!(space, body_param)) >>
    disposition: opt_opt!(preceded!(space, body_disposition)) >>
    lang: opt_opt!(preceded!(space, body_lang)) >>
    location: opt_opt!(preceded!(space, nstring_utf8)) >>
    extension: opt!(preceded!(space, body_extension)) >>
    (BodyStructure::Multipart(BodyStructureMultipart {
        bodies,
        media_subtype,
        param,
        disposition,
        lang,
        location,
        extension
    }))
));

named!(pub body<BodyStructure>, paren_delimited!(
    alt!(body_type_text | body_type_message | body_type_basic | body_type_multipart)
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

    // body-fld-param SP body-fld-id SP body-fld-desc SP body-fld-enc SP body-fld-octets
    const BODY_FIELDS: &str = r#"("foo" "bar") "id" "desc" "7BIT" 1337"#;
    const BODY_FIELD_PARAM_PAIR: BodyParam = BodyParam { key: "foo", val: "bar" };
    const BODY_FIELD_ID: Option<&str> = Some("id");
    const BODY_FIELD_DESC: Option<&str> = Some("desc");
    const BODY_FIELD_ENC: &str = "7BIT";
    const BODY_FIELD_OCTETS: u32 = 1337;

    fn mock_body_text() -> (String, BodyStructureText<'static>) {
        (
            format!(r#"("TEXT" "PLAIN" {} 42)"#, BODY_FIELDS),
            BodyStructureText {
                media_subtype: "PLAIN",
                param: Some(vec![BODY_FIELD_PARAM_PAIR]),
                encoding: BODY_FIELD_ENC,
                octets: BODY_FIELD_OCTETS,
                id: BODY_FIELD_ID,
                description: BODY_FIELD_DESC,
                lines: 42,
                md5: None,
                lang: None,
                location: None,
                extension: None,
                disposition: None,
            }
        )
    }

    #[test]
    fn test_body_param_nil() {
        assert_eq!(
            body_param(br#"NIL"#).unwrap(),
            (EMPTY, None)
        )
    }

    #[test]
    fn test_body_param() {
        assert_eq!(
            body_param(br#"("foo" "bar")"#).unwrap(),
            (EMPTY, Some(vec![BodyParam { key: "foo", val: "bar" }]))
        )
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
            (
                EMPTY,
                Some(BodyDisposition {
                    disposition_type: "attachment",
                    param: Some(vec![BodyParam {
                        key: "FILENAME",
                        val: "pages.pdf"
                    }])
                })
            )
        )
    }

    #[test]
    fn test_body_structure_text() {
        let (body_str, body_struct) = mock_body_text();
        match body(body_str.as_bytes()) {
            Ok((_, BodyStructure::Text(text))) => {
                assert_eq!(text, body_struct)
            }
            rsp @ _ => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_body_structure_text_with_ext() {
        let body_str = format!(r#"("TEXT" "PLAIN" {} 42 NIL NIL NIL NIL)"#, BODY_FIELDS);
        match body(body_str.as_bytes()) {
            Ok((_, BodyStructure::Text(text))) => {
                assert_eq!(text, BodyStructureText {
                    media_subtype: "PLAIN",
                    lines: 42,
                    param: Some(vec![BODY_FIELD_PARAM_PAIR]),
                    encoding: BODY_FIELD_ENC,
                    octets: BODY_FIELD_OCTETS,
                    id: BODY_FIELD_ID,
                    description: BODY_FIELD_DESC,
                    md5: None,
                    lang: None,
                    location: None,
                    extension: None,
                    disposition: None,
                })
            }
            rsp @ _ => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_body_structure_basic() {
        const BODY: &[u8] = br#"("APPLICATION" "PDF" ("NAME" "pages.pdf") NIL NIL "BASE64" 38838 NIL ("attachment" ("FILENAME" "pages.pdf")) NIL NIL)"#;
        match body(BODY) {
            Ok((_, BodyStructure::Basic(basic))) => {
                assert_eq!(basic, BodyStructureBasic {
                    media_type: "APPLICATION",
                    media_subtype: "PDF",
                    param: Some(vec![BodyParam { key: "NAME", val: "pages.pdf" }]),
                    encoding: "BASE64",
                    octets: 38838,
                    disposition: Some(BodyDisposition {
                        disposition_type: "attachment",
                        param: Some(vec![BodyParam { key: "FILENAME", val: "pages.pdf" }])
                    }),
                    id: None,
                    md5: None,
                    lang: None,
                    location: None,
                    extension: None,
                    description: None,
                })
            }
            rsp @ _ => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_body_structure_message() {
        let (text_body_str, _) = mock_body_text();
        let envelope_str = r#"("Wed, 17 Jul 1996 02:23:25 -0700 (PDT)" "IMAP4rev1 WG mtg summary and minutes" (("Terry Gray" NIL "gray" "cac.washington.edu")) (("Terry Gray" NIL "gray" "cac.washington.edu")) (("Terry Gray" NIL "gray" "cac.washington.edu")) ((NIL NIL "imap" "cac.washington.edu")) ((NIL NIL "minutes" "CNRI.Reston.VA.US") ("John Klensin" NIL "KLENSIN" "MIT.EDU")) NIL NIL "<B27397-0100000@cac.washington.edu>")"#;
        let body_str = format!(r#"("MESSAGE" "RFC822" {} {} {} 42)"#, BODY_FIELDS, envelope_str, text_body_str);
        match body(body_str.as_bytes()) {
            Ok((_, BodyStructure::Message(_))) => {},
            rsp @ _ => panic!("unexpected response {:?}", rsp),
        }
    }

    #[test]
    fn test_body_structure_multipart() {
        let (text_body_str1, text_body_struct1) = mock_body_text();
        let (text_body_str2, text_body_struct2) = mock_body_text();
        let body_str = format!(
            r#"({}{} "ALTERNATIVE" NIL NIL NIL NIL)"#,
            text_body_str1, text_body_str2
        );
        match body(body_str.as_bytes()) {
            Ok((_, BodyStructure::Multipart(multipart))) => {
                assert_eq!(multipart, BodyStructureMultipart {
                    bodies: vec![
                        BodyStructure::Text(text_body_struct1),
                        BodyStructure::Text(text_body_struct2),
                    ],
                    media_subtype: "ALTERNATIVE",
                    param: None,
                    lang: None,
                    disposition: None,
                    location: None,
                    extension: None
                })
            }
            rsp @ _ => panic!("unexpected response {:?}", rsp),
        }
    }
}
