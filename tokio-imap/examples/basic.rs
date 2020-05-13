use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io;

use futures::stream::TryStreamExt;
use tokio_imap::builders::CommandBuilder;
use tokio_imap::ResponseData;
use tokio_imap::types::{Attribute, AttributeValue, Response};
use tokio_imap::TlsClient;

#[tokio::main]
async fn main() {
    let mut args = std::env::args();
    let _ = args.next();
    let server = args.next().expect("no server provided");
    let login = args.next().expect("no login provided");
    let password = args.next().expect("no password provided");
    let mailbox = args.next().expect("no mailbox provided");

    if let Err(cause) = imap_fetch(&server, login, password, mailbox).await {
        eprintln!("Fatal error: {}", cause);
    }
}

async fn imap_fetch(
    server: &str,
    login: String,
    password: String,
    mailbox: String,
) -> Result<(), ImapError> {
    eprintln!("Will connect to {}", server);
    let (_, mut tls_client) = TlsClient::connect(server)
        .await
        .map_err(|e| ImapError::Connect { cause: e })?;

    let responses = tls_client
        .call(CommandBuilder::login(&login, &password))
        .try_collect::<Vec<_>>()
        .await
        .map_err(|e| ImapError::Login { cause: e })?;

    match responses[0].parsed() {
        Response::Capabilities(_) => {}
        Response::Done { information, .. } => {
            if let Some(info) = information {
                eprintln!("Login failed: {:?}", info);
            }
            return Err(ImapError::Login {
                cause: io::Error::new(io::ErrorKind::Other, "login failed"),
            });
        }
        _ => unimplemented!(),
    }

    let _ = tls_client
        .call(CommandBuilder::select(&mailbox))
        .try_collect::<Vec<_>>()
        .await
        .map_err(|e| ImapError::Select { cause: e })?;

    let cmd = CommandBuilder::uid_fetch()
        .range_from(1_u32..)
        .attr(Attribute::Uid)
        .attr(Attribute::Rfc822);
    tls_client
        .call(cmd)
        .try_for_each(process_email)
        .await
        .map_err(|e| ImapError::UidFetch { cause: e })?;

    let _ = tls_client
        .call(CommandBuilder::close())
        .try_collect::<Vec<_>>()
        .await
        .map_err(|e| ImapError::Close { cause: e })?;

    eprintln!("Finished fetching messages");
    Ok(())
}

async fn process_email(response_data: ResponseData) -> Result<(), io::Error> {
    if let Response::Fetch(_, ref attr_vals) = *response_data.parsed() {
        for val in attr_vals.iter() {
            match *val {
                AttributeValue::Uid(u) => {
                    eprintln!("Message UID: {}", u);
                }
                AttributeValue::Rfc822(Some(src)) => {
                    eprintln!("Message length: {}", src.to_vec().len());
                }
                _ => (),
            }
        }
    }
    Ok(())
}

#[derive(Debug)]
pub enum ImapError {
    Connect { cause: io::Error },
    Login { cause: io::Error },
    Select { cause: io::Error },
    UidFetch { cause: io::Error },
    Close { cause: io::Error },
}

impl Error for ImapError {
    fn description(&self) -> &'static str {
        ""
    }

    fn cause(&self) -> Option<&dyn Error> {
        match *self {
            ImapError::Connect { ref cause }
            | ImapError::Login { ref cause }
            | ImapError::Select { ref cause }
            | ImapError::UidFetch { ref cause }
            | ImapError::Close { ref cause } => Some(cause),
        }
    }
}

impl Display for ImapError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            ImapError::Connect { ref cause } => write!(f, "Connect failed: {}", cause),
            ImapError::Login { ref cause } => write!(f, "Login failed: {}", cause),
            ImapError::Select { ref cause } => write!(f, "Mailbox selection failed: {}", cause),
            ImapError::UidFetch { ref cause } => write!(f, "Fetching messages failed: {}", cause),
            ImapError::Close { ref cause } => write!(f, "Closing failed: {}", cause),
        }
    }
}
