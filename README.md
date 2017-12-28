# imap-proto

[![Build status](https://api.travis-ci.org/djc/imap-proto.svg?branch=master)](https://travis-ci.org/djc/imap-proto)
[![MIT license](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE-MIT)
[![Apache License 2.0](https://img.shields.io/badge/license-ALv2-blue.svg)](./LICENSE-APACHE)
[![crates.io, downloads](https://img.shields.io/crates/d/imap-proto.svg)](https://crates.io/crates/imap-proto)
[![crates.io, latest release](https://img.shields.io/crates/v/imap-proto.svg)](https://crates.io/crates/imap-proto)
[![API docs, latest release](https://docs.rs/imap-proto/badge.svg)](http://docs.rs/imap-proto)
[![Join the chat at https://gitter.im/djc/tokio-imap](https://badges.gitter.im/Join%20Chat.svg)](https://gitter.im/djc/tokio-imap)

imap-proto is a low-level IMAP protocol support crate, using the type system to
provide a safe API.

Protocol support is implemented in three parts:

* Types that attempt to closely reflect specification requirements
* A parser implementation to help consume protocol messages
* Builder types to help produce protocol messages

imap-proto was initially started as part of [tokio-imap][tokio-imap].
It was moved into a separate crate so that different protocol implementations
can share it as common infrastructure (as proposed by [rust-imap][rust-imap] contributors).
The code tries to closely follow the [IMAP4rev1 RFC][rfc3501], plus extensions.

All feedback welcome. Feel free to file bugs, requests for documentation and
any other feedback in the [issue tracker][issues], or [tweet me][twitter].

[rfc3501]: https://tools.ietf.org/html/rfc3501
[tokio-imap]: https://github.com/djc/tokio-imap
[rust-imap]: https://github.com/mattnenterprise/rust-imap
[issues]: https://github.com/djc/imap-proto/issues
[twitter]: https://twitter.com/djco/

## Progress

- [ ] Client
    - [ ] Parser: many common server responses implemented
    - [ ] Types: most common types implemented
    - [ ] Message builder: most common commands implemented
- [ ] Server
    - [ ] Parser: not started
    - [ ] Types: not started
    - [ ] Message builder: not started
