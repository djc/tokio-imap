# imap-proto and tokio-imap

[![Build status](https://github.com/djc/tokio-imap/workflows/CI/badge.svg)](https://github.com/djc/tokio-imap/actions?query=workflow%3ACI)
[![Join the chat at https://gitter.im/djc/tokio-imap](https://badges.gitter.im/Join%20Chat.svg)](https://gitter.im/djc/tokio-imap?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)
[![MIT license](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE-MIT)
[![Apache License 2.0](https://img.shields.io/badge/license-ALv2-blue.svg)](./LICENSE-APACHE)

All feedback welcome. Feel free to file bugs, requests for documentation and
any other feedback to the [issue tracker][issues] or [tweet me][twitter].

tokio-imap and imap-proto are maintained by Dirkjan Ochtman. If you depend on these
projects, please support the project via [GitHub Sponsors] or contact me for support.

[issues]: https://github.com/djc/tokio-imap/issues
[twitter]: https://twitter.com/djco/
[GitHub Sponsors]: https://github.com/sponsors/djc

## tokio-imap: futures-based IMAP client

[![crates.io, downloads](https://img.shields.io/crates/d/tokio-imap.svg)](https://crates.io/crates/tokio-imap)
[![crates.io, latest release](https://img.shields.io/crates/v/tokio-imap.svg)](https://crates.io/crates/tokio-imap)
[![API docs, latest release](https://docs.rs/tokio-imap/badge.svg)](http://docs.rs/tokio-imap)

NOTE: Unlike imap-proto, tokio-imap doesn't receive much maintenance. As
an alternative we suggest to evaluate async-imap instead.

A [Tokio stack-based][Tokio_stack], fully asynchronous IMAP library, with strong focus on following
the relevant specs, mainly [IMAP4rev1][rfc3501], but with limited support for
the [Conditional STORE][rfc4551] extension. The type system is used to help
enforce correctness where possible. So far, there is only client code and lots
of infrastructure that supposedly could be shared -- no server yet. (If you
want a tokio-based server, look at [IMAPServer][IMAPServer].)

### Feature highlights

* Fully asynchronous by using [tokio-core][tokio-core] and [tokio-io][tokio-io]
* Uses the type system to help enforce correct operation according to spec
* [nom][nom]-based parser (in imap-proto), so far only used for server response messages

### Limitations

* Alpha-level implementation -- no tests yet, limited protocol coverage
* Server is totally unimplemented at this stage

[rfc3501]: https://tools.ietf.org/html/rfc3501
[rfc4551]: https://tools.ietf.org/html/rfc4551
[IMAPServer]: https://github.com/Nordgedanken/IMAPServer-rs
[docs]: https://docs.rs/tokio-imap
[tokio-core]: https://github.com/tokio-rs/tokio-core
[tokio-io]: https://github.com/tokio-rs/tokio-io
[Tokio_stack]: https://tokio.rs
[nom]: https://github.com/Geal/nom

### How to get started

Have a look at the [mailsync][mailsync] crate for example usage.

[mailsync]: https://github.com/djc/mailsync

## imap-proto: IMAP types and protocol parser

[![crates.io, downloads](https://img.shields.io/crates/d/imap-proto.svg)](https://crates.io/crates/imap-proto)
[![crates.io, latest release](https://img.shields.io/crates/v/imap-proto.svg)](https://crates.io/crates/imap-proto)
[![API docs, latest release](https://docs.rs/imap-proto/badge.svg)](http://docs.rs/imap-proto)

imap-proto is a low-level IMAP protocol support crate, using the type system to
provide a safe API. It was extracted from tokio-imap into a separate crate so that
different protocol implementations can share it as common infrastructure
(as proposed by [rust-imap][rust-imap] contributors).
The code tries to closely follow the [IMAP4rev1 RFC][rfc3501], plus extensions.

Protocol support is implemented in three parts:

* Types that attempt to closely reflect specification requirements
* A parser implementation to help consume protocol messages
* Builder types to help produce protocol messages

[rfc3501]: https://tools.ietf.org/html/rfc3501
[tokio-imap]: https://github.com/djc/tokio-imap
[rust-imap]: https://github.com/mattnenterprise/rust-imap
[issues]: https://github.com/djc/imap-proto/issues
[twitter]: https://twitter.com/djco/

### Progress

- [ ] Client
    - [ ] Parser: many common server responses implemented
    - [ ] Types: most common types implemented
    - [ ] Message builder: most common commands implemented
- [ ] Server
    - [ ] Parser: not started
    - [ ] Types: not started
    - [ ] Message builder: not started
