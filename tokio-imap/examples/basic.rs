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

fn main() {
    let mut args = std::env::args();
    let _ = args.next();
    let server = args.next().expect("no server provided");
    let login = args.next().expect("no login provided");
    let password = args.next().expect("no password provided");
    let mailbox = args.next().expect("no mailbox provided");
    if let Err(cause) = imap_fetch(&server, login, password, mailbox) {
        eprintln!("Fatal error: {}", cause);
    };
}

fn imap_fetch(
    server: &str, login: String, password: String, mailbox: String
) -> Result<(), ImapError> {
    eprintln!("Will connect to {}", server);
    let fut_connect = connect(server).map_err(|cause| ImapError::Connect { cause })?;
    let fut_responses = fut_connect
        .and_then(move |(tls_client, _)| {
            tls_client
                .call(CommandBuilder::login(&login, &password))
                .collect()
        })
        .and_then(move |(_, tls_client)| {
            tls_client.call(CommandBuilder::select(&mailbox)).collect()
        })
        .and_then(move |(_, tls_client)| {
            let cmd = CommandBuilder::uid_fetch()
                .all_after(1_u32)
                .attr(Attribute::Uid)
                .attr(Attribute::Rfc822);
            tls_client.call(cmd.build()).for_each(move |response_data| {
                process_email(&response_data);
                Ok(())
            })
        })
        .and_then(move |tls_client| tls_client.call(CommandBuilder::close()).collect())
        .and_then(|_| Ok(()))
        .map_err(|_| ());
    current_thread::run(|_| {
        eprintln!("Fetching e-mails ... ");
        current_thread::spawn(fut_responses);
    });
    eprintln!("Finished fetching e-mails. ");
    Ok(())
}

fn process_email(response_data: &ResponseData) {
    if let Response::Fetch(_, ref attr_vals) = *response_data.parsed() {
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
