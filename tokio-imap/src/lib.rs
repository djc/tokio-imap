#[macro_use]
extern crate futures;

pub mod client;
pub mod proto;

pub use crate::client::{Client, TlsClient};

pub mod types {
    pub use imap_proto::types::*;
}
