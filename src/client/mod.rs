use futures::{Async, Future, Poll, Sink};
use futures::stream::Stream;
use futures::sink::Send;
use futures_state_stream::{StateStream, StreamEvent};

use native_tls::TlsConnector;

use std::io;
use std::net::ToSocketAddrs;

use tokio_core::net::{TcpStream, TcpStreamNew};
use tokio_io::AsyncRead;
use tokio_core::reactor::Handle;
use tokio_tls::{ConnectAsync, TlsConnectorExt};

use imap_proto::*;
use imap_proto::builders::command::Command;
use proto::*;

pub mod builder {
    pub use imap_proto::builders::command::*;
}


pub struct Client {
    transport: ImapTransport,
    state: ClientState,
}

impl Client {
    pub fn connect(server: &str, handle: &Handle) -> ConnectFuture {
        let addr = format!("{}:993", server);
        let addr = addr.to_socket_addrs().unwrap().next().unwrap();
        let stream = TcpStream::connect(&addr, handle);
        ConnectFuture::TcpConnecting(stream, server.to_string())
    }

    pub fn call(self, cmd: Command) -> ResponseStream {
        let Self { transport, mut state } = self;
        let request_id = state.request_ids.next().unwrap();
        let (cmd_bytes, next_state) = cmd.to_parts();
        let future = transport.send(Request(request_id.clone(), cmd_bytes));
        ResponseStream::new(future, state, request_id, next_state)
    }
}

pub struct ResponseStream {
    future: Option<Send<ImapTransport>>,
    transport: Option<ImapTransport>,
    state: Option<ClientState>,
    request_id: RequestId,
    next_state: Option<State>,
    done: bool,
}

impl ResponseStream {
    pub fn new(future: Send<ImapTransport>, state: ClientState,
               request_id: RequestId, next_state: Option<State>) -> Self {
        Self {
            future: Some(future),
            transport: None,
            state: Some(state),
            request_id: request_id,
            next_state: next_state,
            done: false,
        }
    }
}

impl StateStream for ResponseStream {
    type Item = ResponseData;
    type State = Client;
    type Error = io::Error;
    fn poll(&mut self) -> Poll<StreamEvent<Self::Item, Self::State>, Self::Error> {
        if self.future.is_some() {
            let mut future = self.future.take().unwrap();
            match future.poll() {
                Ok(Async::Ready(transport)) => {
                    self.transport = Some(transport);
                },
                Ok(Async::NotReady) => {
                    self.future = Some(future);
                    return Ok(Async::NotReady);
                },
                Err(e) => {
                    return Err(e);
                },
            }
        }
        if !self.transport.is_some() {
            return Ok(Async::NotReady);
        }
        let mut transport = self.transport.take().unwrap();
        if self.done {
            let mut state = self.state.take().unwrap();
            if self.next_state.is_some() {
                state.state = self.next_state.take().unwrap();
            }
            let client = Client { transport, state };
            return Ok(Async::Ready(StreamEvent::Done(client)));
        }
        loop {
            match transport.poll() {
                Ok(Async::Ready(Some(rsp))) => {
                    if let Some(req_id) = rsp.request_id() {
                        self.done = *req_id == self.request_id;
                    };
                    self.transport = Some(transport);
                    return Ok(Async::Ready(StreamEvent::Next(rsp)));
                },
                Ok(Async::Ready(None)) | Ok(Async::NotReady) => {
                    break;
                },
                Err(e) => {
                    return Err(e);
                },
            }
        }
        self.transport = Some(transport);
        Ok(Async::NotReady)
    }
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
    type Item = (Client, ResponseData);
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut new = None;
        if let ConnectFuture::TcpConnecting(ref mut future, ref domain) = *self {
            let stream = try_ready!(future.poll());
            let ctx = TlsConnector::builder().unwrap().build().unwrap();
            let future = ctx.connect_async(domain, stream);
            new = Some(ConnectFuture::TlsHandshake(future));
        }
        if new.is_some() {
            *self = new.take().unwrap();
        }
        if let ConnectFuture::TlsHandshake(ref mut future) = *self {
            let transport = try_ready!(future.map_err(|e| {
                io::Error::new(io::ErrorKind::Other, e)
            }).poll()).framed(ImapCodec::default());
            new = Some(ConnectFuture::ServerGreeting(Some(transport)));
        }
        if new.is_some() {
            *self = new.take().unwrap();
        }
        if let ConnectFuture::ServerGreeting(ref mut wrapped) = *self {
            let msg = try_ready!(wrapped.as_mut().unwrap().poll()).unwrap();
            return Ok(Async::Ready((Client {
                transport: wrapped.take().unwrap(),
                state: ClientState::new(),
            }, msg)));
        }
        Ok(Async::NotReady)
    }
}

pub struct ClientState {
    state: State,
    request_ids: IdGenerator,
}

impl ClientState {
    pub fn new() -> Self {
        Self {
            state: State::NotAuthenticated,
            request_ids: IdGenerator::new(),
        }
    }
}

impl Default for ClientState {
    fn default() -> Self { Self::new() }
}

pub struct IdGenerator {
    next: u64,
}

impl IdGenerator {
    pub fn new() -> Self {
        Self { next: 0 }
    }
}

impl Default for IdGenerator {
    fn default() -> Self { Self::new() }
}

impl Iterator for IdGenerator {
    type Item = RequestId;
    fn next(&mut self) -> Option<Self::Item> {
        self.next += 1;
        Some(RequestId(format!("A{:04}", self.next % 10_000)))
    }
}
