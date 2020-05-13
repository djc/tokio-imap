mod client;
mod codec;

pub use crate::client::{Client, TlsClient};
pub use crate::codec::ResponseData;

pub mod builders {
    pub use imap_proto::builders::command::{fetch, CommandBuilder, FetchCommand};
}

pub mod types {
    pub use imap_proto::types::*;
}
