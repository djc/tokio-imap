extern crate futures;
extern crate futures_state_stream;
extern crate tokio;
extern crate tokio_imap;

use futures::future::Future;
use futures_state_stream::StateStream;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io;
use tokio::executor::current_thread;
use tokio_imap::ImapClient;
use tokio_imap::client::builder::{CommandBuilder, FetchBuilderAttributes, FetchBuilderMessages,
                                  FetchBuilderModifiers};
use tokio_imap::client::connect;
use tokio_imap::proto::ResponseData;
use tokio_imap::types::{Attribute, AttributeValue, Response};

fn process_email(responsedata: &ResponseData) {
    if let Response::Fetch(_, ref attr_vals) = *responsedata.parsed() {
        for val in attr_vals.iter() {
            match *val {
                AttributeValue::Uid(u) => {
                    eprintln!("E-mail UID: {}", u);
                },
                AttributeValue::Rfc822(Some(src)) => {
                    eprintln!("E-mail body length: {}", src.to_vec().len());
                },
                _ => (),
            }
        }
    }
}

#[derive(Debug)]
pub enum ImapError {
    Connect { cause: io::Error },
    Login { cause: io::Error },
    Select { cause: io::Error },
    UidFetch { cause: io::Error },
}

impl Error for ImapError {
    fn description(&self) -> &'static str {
        ""
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            ImapError::Connect { ref cause }
            | ImapError::Login { ref cause }
            | ImapError::Select { ref cause }
            | ImapError::UidFetch { ref cause } => Some(cause),
        }
    }
}

impl Display for ImapError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            ImapError::Connect { ref cause } => write!(f, "Connect failed: {}. ", cause),
            ImapError::Login { ref cause } => write!(f, "Login failed: {}. ", cause),
            ImapError::Select { ref cause } => write!(f, "Mailbox selection failed: {}. ", cause),
            ImapError::UidFetch { ref cause } => write!(f, "Fetching e-mails failed: {}. ", cause),
        }
    }
}

fn imapfetch(
    server: &str, login: String, password: String, mailbox: String
) -> Result<(), ImapError> {
    eprintln!("Will connect to {}", server);
    let fut_connect = connect(server).map_err(|cause| ImapError::Connect { cause })?;
    let fut_responses = fut_connect
        .and_then(move |(tlsclient, _)| {
            tlsclient
                .call(CommandBuilder::login(login.as_str(), password.as_str()))
                .collect()
        })
        .and_then(move |(_, tlsclient)| {
            tlsclient
                .call(CommandBuilder::select(mailbox.as_str()))
                .collect()
        })
        .and_then(move |(_, tlsclient)| {
            let cmd = CommandBuilder::uid_fetch()
                .all_after(1_u32)
                .attr(Attribute::Uid)
                .attr(Attribute::Rfc822);
            tlsclient.call(cmd.build()).for_each(move |responsedata| {
                process_email(&responsedata);
                Ok(())
            })
        })
        .and_then(move |tlsclient: tokio_imap::TlsClient| {
            tlsclient.call(CommandBuilder::close()).collect()
        })
        .and_then(|_| Ok(()))
        .map_err(|_| ());
    current_thread::run(|_| {
        eprintln!("Fetching e-mails ... ");
        current_thread::spawn(fut_responses);
    });
    eprintln!("Finished fetching e-mails. ");
    Ok(())
}

fn main() {
    // Provide server address, login, password and mailbox name on standard input, each on a line
    // and 4 lines in total.
    let (mut server, mut login, mut password, mut mailbox) =
        (String::new(), String::new(), String::new(), String::new());
    let (server, login, password, mailbox) = {
        io::stdin()
            .read_line(&mut server)
            .expect("Provide an IMAP server FQDN. ");
        io::stdin()
            .read_line(&mut login)
            .expect("Provide a login. ");
        io::stdin()
            .read_line(&mut password)
            .expect("Provide a password. ");
        io::stdin()
            .read_line(&mut mailbox)
            .expect("Provide a mailbox. ");
        (
            server.trim(),
            login.trim().to_owned(),
            password.trim().to_owned(),
            mailbox.trim().to_owned(),
        )
    };
    if let Err(cause) = imapfetch(server, login, password, mailbox) {
        eprintln!("Fatal error: {}", cause);
    };
}
