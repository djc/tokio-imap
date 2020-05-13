mod client;
mod codec;

pub use crate::client::builder::CommandBuilder;
pub use crate::client::{Client, TlsClient};
pub use crate::codec::ResponseData;

pub mod types {
    pub use imap_proto::types::*;
}
