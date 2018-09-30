use types::*;
use core::*;

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

named!(pub section<Option<SectionPath>>, do_parse!(
    tag_s!("[") >>
    spec: opt!(section_spec) >>
    tag_s!("]") >>
    (spec)
));

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



// ======================= TBD ===========================


// body            = "(" (body-type-1part / body-type-mpart) ")"
// 
// body-type-1part = (body-type-basic / body-type-msg / body-type-text) [SP body-ext-1part]
// body-ext-1part = body-fld-md5 [SP body-fld-dsp [SP body-fld-lang [SP body-fld-loc *(SP body-extension)]]]
//                   ; MUST NOT be returned on non-extensible
//                   ; "BODY" fetch

// body-type-basic = media-basic SP body-fields ; MESSAGE subtype MUST NOT be "RFC822"
// body-type-msg   = media-message SP body-fields SP envelope SP body SP body-fld-lines
// body-type-text  = media-text SP body-fields SP body-fld-lines
// 
// body-type-mpart = 1*body SP media-subtype [SP body-ext-mpart]
// body-ext-mpart = body-fld-param [SP body-fld-dsp [SP body-fld-lang [SP body-fld-loc *(SP body-extension)]]]
//                   ; MUST NOT be returned on non-extensible
//                   ; "BODY" fetch


// body-extension  = nstring / number / "(" body-extension *(SP body-extension) ")"
//                    ; Future expansion.  Client implementations
//                    ; MUST accept body-extension fields.  Server
//                    ; implementations MUST NOT generate
//                    ; body-extension fields except as defined by
//                    ; future standard or standards-track
//                    ; revisions of this specification.


// media-basic     = ((DQUOTE ("APPLICATION" / "AUDIO" / "IMAGE" / "MESSAGE" / "VIDEO") DQUOTE) / string) SP media-subtype
//                     ; Defined in [MIME-IMT]
// media-message   = DQUOTE "MESSAGE" DQUOTE SP DQUOTE "RFC822" DQUOTE
//                     ; Defined in [MIME-IMT]
// media-subtype   = string
//                     ; Defined in [MIME-IMT]
// media-text      = DQUOTE "TEXT" DQUOTE SP media-subtype
//                     ; Defined in [MIME-IMT]



// body-fields     = body-fld-param SP body-fld-id SP body-fld-desc SP
//                   body-fld-enc SP body-fld-octets
// body-fld-param  = "(" string SP string *(SP string SP string) ")" / nil
// body-fld-id     = nstring
// body-fld-desc   = nstring
// body-fld-enc    = (DQUOTE 
//                   ("7BIT" / "8BIT" / "BINARY" / "BASE64"/ "QUOTED-PRINTABLE")
//                   DQUOTE) /
//                   string
// body-fld-octets = number
// body-fld-dsp    = "(" string SP body-fld-param ")" / nil
// body-fld-lang   = nstring / "(" string *(SP string) ")"
// body-fld-loc    = nstring
// body-fld-md5    = nstring
// body-fld-lines  = number











