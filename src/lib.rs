extern crate bytes;
#[macro_use]
extern crate futures;
extern crate native_tls;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_tls;
extern crate tokio_proto;

use std::io;
use std::net::ToSocketAddrs;
use std::str;

use bytes::{BufMut, BytesMut};
use futures::{Async, Future, Poll, Sink};
use futures::stream::Stream;
use futures::sink::Send;
use native_tls::{TlsConnector};
use tokio_core::net::{TcpStream, TcpStreamNew};
use tokio_io::AsyncRead;
use tokio_io::codec::{Decoder, Encoder, Framed};
use tokio_core::reactor::Handle;
use tokio_tls::{ConnectAsync, TlsConnectorExt, TlsStream};

pub struct ImapCodec;

impl Decoder for ImapCodec {
    type Item = String;
    type Error = io::Error;
    fn decode(&mut self, buf: &mut BytesMut)
             -> Result<Option<String>, io::Error> {
        if let Some(n) = buf.iter().position(|b| *b == b'\n') {
            let line = buf.split_to(n);
            buf.split_to(1);
            return match str::from_utf8(line.get(..).unwrap()) {
                Ok(s) => Ok(Some(s.to_string())),
                Err(_) => Err(io::Error::new(io::ErrorKind::Other,
                                             "invalid string")),
            }
        } else {
            Ok(None)
        }
    }
}

impl Encoder for ImapCodec {
    type Item = String;
    type Error = io::Error;
    fn encode(&mut self, msg: String, dst: &mut BytesMut) -> Result<(), io::Error> {
        dst.put(msg.as_bytes());
        dst.put("\r\n");
        Ok(())
    }
}

type ImapTransport = Framed<TlsStream<TcpStream>, ImapCodec>;

enum ProtoState {
    NotAuthenticated,
    Authenticated,
    Selected,
    Logout,
}

pub struct ClientState {
    state: ProtoState,
    server_greeting: String,
}

pub struct Client {
    transport: ImapTransport,
    state: ClientState,
}

pub enum ConnectFuture {
    #[doc(hidden)]
    TcpConnecting(TcpStreamNew, String),
    #[doc(hidden)]
    TlsHandshake(ConnectAsync<TcpStream>),
    #[doc(hidden)]
    ServerGreeting(Option<ImapTransport>),
}

impl Future for ConnectFuture {
    type Item = Client;
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut changed = true;
        while changed {
            changed = false;
            let fut = match *self {
                ConnectFuture::TcpConnecting(ref mut future, ref domain) => {
                    let stream = try_ready!(future.poll());
                    let ctx = TlsConnector::builder().unwrap().build().unwrap();
                    let future = ctx.connect_async(&domain, stream);
                    changed = true;
                    ConnectFuture::TlsHandshake(future)
                },
                ConnectFuture::TlsHandshake(ref mut future) => {
                    let transport = try_ready!(future.map_err(|e| {
                        io::Error::new(io::ErrorKind::Other, e)
                    }).poll()).framed(ImapCodec);
                    changed = true;
                    ConnectFuture::ServerGreeting(Some(transport))
                },
                ConnectFuture::ServerGreeting(ref mut wrapped) => {
                    println!("server greeting");
                    let mut transport = wrapped.take().unwrap();
                    let msg = try_ready!(transport.poll()).unwrap();
                    return Ok(Async::Ready(Client {
                        transport: transport,
                        state: ClientState {
                            state: ProtoState::NotAuthenticated,
                            server_greeting: msg,
                        },
                    }));
                },
            };
            *self = fut;
        }
        Ok(Async::NotReady)
    }
}

pub struct LoginFuture {
    future: Send<ImapTransport>,
    clst: Option<ClientState>,
}

impl Future for LoginFuture {
    type Item = Client;
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        println!("login?");
        let transport = try_ready!(self.future.poll());
        let mut state = self.clst.take().unwrap();
        state.state = ProtoState::Authenticated;
        return Ok(Async::Ready(Client {
            transport: transport,
            state: state,
        }));
    }
}

impl Client {
    pub fn connect(server: &str, handle: &Handle) -> ConnectFuture {
        let addr = format!("{}:993", server);
        let addr = addr.to_socket_addrs().unwrap().next().unwrap();
        let stream = TcpStream::connect(&addr, handle);
        ConnectFuture::TcpConnecting(stream, server.to_string())
    }

    pub fn login(self, account: &str, password: &str) -> LoginFuture {
        let Client { transport, state } = self;
        let msg = format!("a001 LOGIN {} {}", account, password);
        LoginFuture {
            future: transport.send(msg),
            clst: Some(state),
        }
    }

    pub fn server_greeting(&self) -> &str {
        &self.state.server_greeting
    }
}
