//!
//! https://tools.ietf.org/html/rfc5161
//!
//! The IMAP ENABLE Extension
//!

// rustfmt doesn't do a very good job on nom parser invocations.
#![cfg_attr(rustfmt, rustfmt_skip)]
#![cfg_attr(feature = "cargo-clippy", allow(redundant_closure))]

use crate::types::*;
use crate::parser::core::atom;

// The ENABLED response lists capabilities that were enabled in response
// to a ENABLE command.
// [RFC5161 - 3.2 The ENABLED Response](https://tools.ietf.org/html/rfc5161#section-3.2)
named!(pub (crate) resp_enabled<Response>, map!(
    enabled_data,
    |c| Response::Capabilities(c)
));

named!(enabled_data<Vec<Capability>>, do_parse!(
        tag_no_case!("ENABLED") >>
        capabilities: many0!(preceded!(char!(' '), capability)) >>
        (capabilities)
));

named!(capability<Capability>,
       map!(atom, |a| Capability::Atom(a))
);
