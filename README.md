# tokio-imap

[![MIT license](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE-MIT)
[![Apache License 2.0](https://img.shields.io/badge/license-ALv2-blue.svg)](./LICENSE-APACHE)

[![crates.io, downloads](https://img.shields.io/crates/d/tokio-imap.svg)](https://crates.io/crates/tokio-imap)
[![crates.io, latest release](https://img.shields.io/crates/v/tokio-imap.svg)](https://crates.io/crates/tokio-imap)

[![API docs, latest release](https://docs.rs/tokio-imap/badge.svg)](http://docs.rs/tokio-imap)
[![Join the chat at https://gitter.im/djc/tokio-imap](https://badges.gitter.im/Join%20Chat.svg)](https://gitter.im/djc/tokio-imap?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

A [Tokio stack-based][Tokio_stack], fully asynchronous IMAP library, with strong focus on following
the relevant specs, mainly [IMAP4rev1][rfc3501], but with limited support for
the [Conditional STORE][rfc4551] extension. The type system is used to help
enforce correctness where possible. So far, there is only client code and lots
of infrastructure that supposedly could be shared -- no server yet. (If you
want a tokio-based server, look at [IMAPServer][IMAPServer].)

All feedback welcome. Feel free to file bugs, requests for documentation and
any other feedback to the [issue tracker][issues] or [tweet me][twitter].

### Feature highlights

* Fully asynchronous by using [tokio-core][tokio-core] and [tokio-io][tokio-io]
* Uses the type system to help enforce correct operation according to spec
* [nom][nom]-based parser, so far only used for server response messages

### Limitations

* Alpha-level implementation -- no tests yet, limited protocol coverage
* Server is totally unimplemented at this stage

[rfc3501]: https://tools.ietf.org/html/rfc3501
[rfc4551]: https://tools.ietf.org/html/rfc4551
[IMAPServer]: https://github.com/Nordgedanken/IMAPServer-rs
[docs]: https://docs.rs/tokio-imap
[issues]: https://github.com/djc/tokio-imap/issues
[twitter]: https://twitter.com/djco/
[tokio-core]: https://github.com/tokio-rs/tokio-core
[tokio-io]: https://github.com/tokio-rs/tokio-io
[Tokio_stack]: https://tokio.rs
[nom]: https://github.com/Geal/nom


How to get started
------------------

Have a look at the [mailsync][mailsync] crate for example usage.

[mailsync]: https://github.com/djc/mailsync
