# tokio-imap

[![Latest version](https://img.shields.io/crates/v/tokio-imap.svg)](https://crates.io/crates/tokio-imap)

A tokio-based, fully asynchronous IMAP library, with strong focus on following
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
[nom]: https://github.com/Geal/nom


How to get started
------------------

Have a look at the [mailsync][mailsync] crate for example usage.

[mailsync]: https://github.com/djc/mailsync
