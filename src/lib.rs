#![deny(warnings)]
#![deny(unused)]
#![deny(future_incompatible)]
#![deny(bad_style)]
#![cfg_attr(feature = "cargo-clippy", deny(clippy))]
#![cfg_attr(feature = "cargo-clippy", deny(clippy_pedantic))]
#![cfg_attr(feature = "cargo-clippy", warn(missing_docs_in_private_items))]

extern crate bytes;
#[macro_use]
extern crate futures;
extern crate futures_state_stream;
extern crate imap_proto;
extern crate native_tls;
extern crate nom;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_tls;

pub mod client;
pub mod proto;

pub use client::Client;

pub mod types {
    pub use imap_proto::types::*;
}
