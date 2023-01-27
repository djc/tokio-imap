use std::borrow::Cow;

use nom::branch::alt;
use nom::bytes::streaming::tag_no_case;
use nom::combinator::map;
use nom::sequence::preceded;
use nom::IResult;

use crate::{AttributeValue, MailboxDatum};

use super::core::{parenthesized_list, quoted_utf8};
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
}
