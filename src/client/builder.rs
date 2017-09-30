use proto::{Attribute, AttrMacro, State};

pub struct CommandBuilder { }

impl CommandBuilder {
    pub fn check() -> Command {
        let mut args = vec![];
        args.extend(b"CHECK");
        Command {
            args: args,
            next_state: None,
        }
    }

    pub fn close() -> Command {
        let args = b"CLOSE".to_vec();
        Command { args, next_state: Some(State::Authenticated) }
    }

    pub fn examine(mailbox: &str) -> Command {
        let mut args = vec![];
        args.extend(b"EXAMINE \"");
        args.extend(mailbox.as_bytes());
        args.push(b'"');
        Command {
            args: args,
            next_state: Some(State::Selected),
        }
    }

    pub fn fetch() -> FetchCommandEmpty {
        let mut args = vec![];
        args.extend(b"FETCH ");
        FetchCommandEmpty { args: args }
    }

    pub fn list(reference: &str, glob: &str) -> Command {
        let mut args = vec![];
        args.extend(format!("LIST \"{}\" \"{}\"", reference, glob).as_bytes());
        Command { args, next_state: None }
    }

    pub fn login(user_name: &str, password: &str) -> Command {
        let mut args = vec![];
        args.extend(b"LOGIN ");
        args.extend(user_name.as_bytes());
        args.push(b' ');
        args.extend(password.as_bytes());
        Command {
            args: args,
            next_state: Some(State::Authenticated),
        }
    }

    pub fn select(mailbox: &str) -> Command {
        let mut args = vec![];
        args.extend(b"SELECT \"");
        args.extend(mailbox.as_bytes());
        args.push(b'"');
        Command {
            args: args,
            next_state: Some(State::Selected),
        }
    }

    pub fn uid_fetch() -> FetchCommandEmpty {
        let mut args = vec![];
        args.extend(b"UID FETCH ");
        FetchCommandEmpty { args }
    }
}

pub struct Command {
    args: Vec<u8>,
    next_state: Option<State>,
}

impl Command {
    pub fn to_parts(self) -> (Vec<u8>, Option<State>) {
        let Command { args, next_state } = self;
        (args, next_state)
    }
}

pub struct FetchCommandEmpty {
    args: Vec<u8>,
}

impl FetchBuilderMessages for FetchCommandEmpty {
    fn prepare(self) -> FetchCommandMessages {
        let FetchCommandEmpty { args } = self;
        FetchCommandMessages { args }
    }
}

pub struct FetchCommandMessages {
    args: Vec<u8>,
}

impl FetchBuilderMessages for FetchCommandMessages {
    fn prepare(self) -> FetchCommandMessages {
        let FetchCommandMessages { mut args } = self;
        args.push(b',');
        FetchCommandMessages { args }
    }
}

impl FetchCommandMessages {
    pub fn attr_macro(self, named: AttrMacro) -> FetchCommand {
        let FetchCommandMessages { mut args } = self;
        args.push(b' ');
        match named {
            AttrMacro::All => { args.extend(b"ALL"); },
            AttrMacro::Fast => { args.extend(b"FAST"); },
            AttrMacro::Full => { args.extend(b"FULL"); },
        }
        FetchCommand { args }
    }
}

pub trait FetchBuilderMessages where Self: Sized {
    fn prepare(self) -> FetchCommandMessages;

    fn num(self, num: u32) -> FetchCommandMessages {
        let FetchCommandMessages { mut args } = self.prepare();
        args.extend(num.to_string().as_bytes());
        FetchCommandMessages { args }
    }

    fn range(self, start: u32, stop: u32) -> FetchCommandMessages {
        let FetchCommandMessages { mut args } = self.prepare();
        args.extend(start.to_string().as_bytes());
        args.push(b':');
        args.extend(stop.to_string().as_bytes());
        FetchCommandMessages { args }
    }

    fn all_after(self, start: u32) -> FetchCommandMessages {
        let FetchCommandMessages { mut args } = self.prepare();
        args.extend(start.to_string().as_bytes());
        args.extend(b":*");
        FetchCommandMessages { args }
    }
}

pub struct FetchCommandAttributes {
    args: Vec<u8>,
}

impl FetchBuilderAttributes for FetchCommandMessages {
    fn prepare(self) -> FetchCommandAttributes {
        let FetchCommandMessages { mut args } = self;
        args.extend(b" (");
        FetchCommandAttributes { args }
    }
}

impl FetchBuilderAttributes for FetchCommandAttributes {
    fn prepare(self) -> FetchCommandAttributes {
        let FetchCommandAttributes { mut args } = self;
        args.push(b' ');
        FetchCommandAttributes { args }
    }
}

pub trait FetchBuilderAttributes where Self: Sized {
    fn prepare(self) -> FetchCommandAttributes;
    fn attr(self, attr: Attribute) -> FetchCommandAttributes {
        let FetchCommandAttributes { mut args } = self.prepare();
        args.extend(match attr {
            Attribute::Body => "BODY",
            Attribute::Envelope => "ENVELOPE",
            Attribute::Flags => "FLAGS",
            Attribute::InternalDate => "INTERNALDATE",
            Attribute::ModSeq => "MODSEQ",
            Attribute::Rfc822 => "RFC822",
            Attribute::Rfc822Size => "RFC822.SIZE",
            Attribute::Uid => "UID",
        }.as_bytes());
        FetchCommandAttributes { args }
    }
}

pub struct FetchCommand {
    args: Vec<u8>,
}

pub trait FetchBuilderModifiers where Self: Sized {
    fn prepare(self) -> FetchCommand;
    fn build(self) -> Command {
        let FetchCommand { args } = self.prepare();
        Command { args, next_state: None }
    }
    fn changed_since(self, seq: u64) -> FetchCommand {
        let FetchCommand { mut args } = self.prepare();
        args.extend(b" (CHANGEDSINCE ");
        args.extend(seq.to_string().as_bytes());
        args.push(b')');
        FetchCommand { args }
    }
}

impl FetchBuilderModifiers for FetchCommandAttributes {
    fn prepare(self) -> FetchCommand {
        let FetchCommandAttributes { mut args, .. } = self;
        args.push(b')');
        FetchCommand { args }
    }
}

impl FetchBuilderModifiers for FetchCommand {
    fn prepare(self) -> FetchCommand { self }
}

