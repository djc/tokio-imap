#![deny(warnings)]
#![deny(unused)]
#![deny(future_incompatible)]
#![deny(bad_style)]

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
