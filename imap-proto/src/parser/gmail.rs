use std::borrow::Cow;

use nom::branch::alt;
use nom::bytes::streaming::tag_no_case;
use nom::combinator::map;
use nom::sequence::preceded;
use nom::IResult;

use crate::{AttributeValue, MailboxDatum};

use super::core::{number_64, parenthesized_list, quoted_utf8};
use super::rfc3501::flag;

pub(crate) fn gmail_label_list(i: &[u8]) -> IResult<&[u8], Vec<Cow<str>>> {
    preceded(
        tag_no_case("X-GM-LABELS "),
        parenthesized_list(map(alt((flag, quoted_utf8)), Cow::Borrowed)),
    )(i)
}

pub(crate) fn msg_att_gmail_labels(i: &[u8]) -> IResult<&[u8], AttributeValue> {
    map(gmail_label_list, AttributeValue::GmailLabels)(i)
}

pub(crate) fn mailbox_data_gmail_labels(i: &[u8]) -> IResult<&[u8], MailboxDatum> {
    map(gmail_label_list, MailboxDatum::GmailLabels)(i)
}

pub(crate) fn gmail_msgid(i: &[u8]) -> IResult<&[u8], u64> {
    preceded(tag_no_case("X-GM-MSGID "), number_64)(i)
}

pub(crate) fn msg_att_gmail_msgid(i: &[u8]) -> IResult<&[u8], AttributeValue> {
    map(gmail_msgid, AttributeValue::GmailMsgId)(i)
}

pub(crate) fn mailbox_data_gmail_msgid(i: &[u8]) -> IResult<&[u8], MailboxDatum> {
    map(gmail_msgid, MailboxDatum::GmailMsgId)(i)
}

#[cfg(test)]
mod tests {
    use crate::types::*;
    #[test]
    fn test_gmail_labels() {
        let env = br#"X-GM-LABELS (\Inbox \Sent Important "Muy Importante") "#;
        match super::msg_att_gmail_labels(env) {
            Ok((_, AttributeValue::GmailLabels(labels))) => {
                println!("{labels:?}");
                assert_eq!(
                    ["\\Inbox", "\\Sent", "Important", "Muy Importante"].to_vec(),
                    labels
                );
            }
            rsp => {
                let e = rsp.unwrap_err();
                if let nom::Err::Error(i) = &e {
                    println!("{:?}", std::str::from_utf8(i.input));
                }
                panic!("unexpected response {e:?}");
            }
        }
    }

    #[test]
    fn test_gmail_msgid() {
        let env = br#"X-GM-MSGID 1278455344230334865 "#;
        match super::msg_att_gmail_msgid(env) {
            Ok((_, AttributeValue::GmailMsgId(msgid))) => {
                println!("{msgid:?}");
                assert_eq!(1278455344230334865u64, msgid);
            }
            rsp => {
                let e = rsp.unwrap_err();
                if let nom::Err::Error(i) = &e {
                    println!("{:?}", std::str::from_utf8(i.input));
                }
                panic!("unexpected response {e:?}");
            }
        }
    }
}
