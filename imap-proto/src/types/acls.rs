use super::to_owned_cow;

use std::borrow::Cow;

// IMAP4 ACL Extension 4313/2086

#[derive(Debug, Eq, PartialEq)]
pub struct Acl<'a> {
    pub mailbox: Cow<'a, str>,
    pub acls: Vec<AclEntry<'a>>,
}

impl<'a> Acl<'a> {
    pub fn into_owned(self) -> Acl<'static> {
        Acl {
            mailbox: to_owned_cow(self.mailbox),
            acls: self.acls.into_iter().map(AclEntry::into_owned).collect(),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct AclEntry<'a> {
    pub identifier: Cow<'a, str>,
    pub rights: Vec<AclRight>,
}

impl<'a> AclEntry<'a> {
    pub fn into_owned(self) -> AclEntry<'static> {
        AclEntry {
            identifier: to_owned_cow(self.identifier),
            rights: self.rights,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ListRights<'a> {
    pub mailbox: Cow<'a, str>,
    pub identifier: Cow<'a, str>,
    pub required: Vec<AclRight>,
    pub optional: Vec<AclRight>,
}

impl<'a> ListRights<'a> {
    pub fn into_owned(self) -> ListRights<'static> {
        ListRights {
            mailbox: to_owned_cow(self.mailbox),
            identifier: to_owned_cow(self.identifier),
            required: self.required,
            optional: self.optional,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct MyRights<'a> {
    pub mailbox: Cow<'a, str>,
    pub rights: Vec<AclRight>,
}

impl<'a> MyRights<'a> {
    pub fn into_owned(self) -> MyRights<'static> {
        MyRights {
            mailbox: to_owned_cow(self.mailbox),
            rights: self.rights,
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum AclRight {
    /// l - lookup (mailbox is visible to LIST/LSUB commands, SUBSCRIBE
    /// mailbox)
    Lookup,
    /// r - read (SELECT the mailbox, perform STATUS)
    Read,
    /// s - keep seen/unseen information across sessions (set or clear
    /// \SEEN flag via STORE, also set \SEEN during APPEND/COPY/
    /// FETCH BODY[...])
    Seen,
    /// w - write (set or clear flags other than \SEEN and \DELETED via
    /// STORE, also set them during APPEND/COPY)
    Write,
    /// i - insert (perform APPEND, COPY into mailbox)
    Insert,
    /// p - post (send mail to submission address for mailbox,
    /// not enforced by IMAP4 itself)
    Post,
    /// k - create mailboxes (CREATE new sub-mailboxes in any
    /// implementation-defined hierarchy, parent mailbox for the new
    /// mailbox name in RENAME)
    CreateMailbox,
    /// x - delete mailbox (DELETE mailbox, old mailbox name in RENAME)
    DeleteMailbox,
    /// t - delete messages (set or clear \DELETED flag via STORE, set
    /// \DELETED flag during APPEND/COPY)
    DeleteMessage,
    /// e - perform EXPUNGE and expunge as a part of CLOSE
    Expunge,
    /// a - administer (perform SETACL/DELETEACL/GETACL/LISTRIGHTS)
    Administer,
    /// n - ability to write .shared annotations values
    /// From RFC 5257
    Annotation,
    /// c - old (deprecated) create. Do not use. Read RFC 4314 for more information.
    OldCreate,
    /// d - old (deprecated) delete. Do not use. Read RFC 4314 for more information.
    OldDelete,
    /// A custom right
    Custom(char),
}

impl From<char> for AclRight {
    fn from(c: char) -> Self {
        match c {
            'l' => AclRight::Lookup,
            'r' => AclRight::Read,
            's' => AclRight::Seen,
            'w' => AclRight::Write,
            'i' => AclRight::Insert,
            'p' => AclRight::Post,
            'k' => AclRight::CreateMailbox,
            'x' => AclRight::DeleteMailbox,
            't' => AclRight::DeleteMessage,
            'e' => AclRight::Expunge,
            'a' => AclRight::Administer,
            'n' => AclRight::Annotation,
            'c' => AclRight::OldCreate,
            'd' => AclRight::OldDelete,
            _ => AclRight::Custom(c),
        }
    }
}

impl From<AclRight> for char {
    fn from(right: AclRight) -> Self {
        match right {
            AclRight::Lookup => 'l',
            AclRight::Read => 'r',
            AclRight::Seen => 's',
            AclRight::Write => 'w',
            AclRight::Insert => 'i',
            AclRight::Post => 'p',
            AclRight::CreateMailbox => 'k',
            AclRight::DeleteMailbox => 'x',
            AclRight::DeleteMessage => 't',
            AclRight::Expunge => 'e',
            AclRight::Administer => 'a',
            AclRight::Annotation => 'n',
            AclRight::OldCreate => 'c',
            AclRight::OldDelete => 'd',
            AclRight::Custom(c) => c,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_to_acl_right() {
        assert_eq!(Into::<AclRight>::into('l'), AclRight::Lookup);
        assert_eq!(Into::<AclRight>::into('c'), AclRight::OldCreate);
        assert_eq!(Into::<AclRight>::into('k'), AclRight::CreateMailbox);
        assert_eq!(Into::<AclRight>::into('0'), AclRight::Custom('0'));
    }

    #[test]
    fn test_acl_right_to_char() {
        assert_eq!(Into::<char>::into(AclRight::Lookup), 'l');
        assert_eq!(Into::<char>::into(AclRight::OldCreate), 'c');
        assert_eq!(Into::<char>::into(AclRight::CreateMailbox), 'k');
        assert_eq!(Into::<char>::into(AclRight::Custom('0')), '0');
    }
}
