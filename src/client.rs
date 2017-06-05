use futures::{Async, Future, Poll, Sink};
use futures::stream::Stream;
use futures::sink::Send;

use native_tls::TlsConnector;

use std::io;
use std::net::ToSocketAddrs;

use tokio_core::net::{TcpStream, TcpStreamNew};
use tokio_io::AsyncRead;
use tokio_core::reactor::Handle;
use tokio_tls::{ConnectAsync, TlsConnectorExt};

use proto;

pub struct ClientState {
    state: proto::State,
    next_request_id: u64,
}

pub struct Client {
    transport: proto::ImapTransport,
    state: ClientState,
}

pub enum ConnectFuture {
    #[doc(hidden)]
    TcpConnecting(TcpStreamNew, String),
    #[doc(hidden)]
    TlsHandshake(ConnectAsync<TcpStream>),
    #[doc(hidden)]
    ServerGreeting(Option<proto::ImapTransport>),
}

impl Future for ConnectFuture {
    type Item = (Client, proto::Response);
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut new = None;
        if let ConnectFuture::TcpConnecting(ref mut future, ref domain) = *self {
            let stream = try_ready!(future.poll());
            let ctx = TlsConnector::builder().unwrap().build().unwrap();
            let future = ctx.connect_async(&domain, stream);
            new = Some(ConnectFuture::TlsHandshake(future));
        }
        if new.is_some() {
            *self = new.take().unwrap();
        }
        if let ConnectFuture::TlsHandshake(ref mut future) = *self {
            let transport = try_ready!(future.map_err(|e| {
                io::Error::new(io::ErrorKind::Other, e)
            }).poll()).framed(proto::ImapCodec);
            new = Some(ConnectFuture::ServerGreeting(Some(transport)));
        }
        if new.is_some() {
            *self = new.take().unwrap();
        }
        if let ConnectFuture::ServerGreeting(ref mut wrapped) = *self {
            let msg = try_ready!(wrapped.as_mut().unwrap().poll()).unwrap();
            return Ok(Async::Ready((Client {
                transport: wrapped.take().unwrap(),
                state: ClientState {
                    state: proto::State::NotAuthenticated,
                    next_request_id: 0,
                },
            }, msg)));
        }
        Ok(Async::NotReady)
    }
}

pub struct CommandFuture {
    future: Option<Send<proto::ImapTransport>>,
    transport: Option<proto::ImapTransport>,
    state: Option<ClientState>,
    request_id: String,
    next_state: Option<proto::State>,
    responses: Option<ServerMessages>,
}

impl CommandFuture {
    fn push_frame(&mut self, frame: proto::Response) {
        match self.responses {
            Some(ref mut responses) => {
                responses.frames.push(frame);
            },
            None => panic!("unpossible"),
        }
    }
}

pub struct ServerMessages {
    pub frames: Vec<proto::Response>,
}

impl ServerMessages {
    fn new() -> ServerMessages {
        ServerMessages { frames: Vec::new() }
    }
}

impl Future for CommandFuture {
    type Item = (Client, ServerMessages);
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
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
        loop {
            match transport.poll() {
                Ok(Async::Ready(Some(proto::Response::Status(Some(req_id), msg)))) => {
                    let expected = req_id == self.request_id;
                    let rsp = proto::Response::Status(Some(req_id), msg);
                    self.push_frame(rsp);
                    if !expected {
                        continue;
                    }
                    let mut state = self.state.take().unwrap();
                    if self.next_state.is_some() {
                        state.state = self.next_state.take().unwrap();
                    }
                    let client = Client { transport, state };
                    let responses = self.responses.take().unwrap();
                    return Ok(Async::Ready((client, responses)));
                },
                Ok(Async::Ready(Some(frame))) => {
                    self.push_frame(frame);
                    continue;
                },
                Ok(Async::Ready(None)) => {
                    break;
                },
                Ok(Async::NotReady) => {
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

impl Client {
    pub fn connect(server: &str, handle: &Handle) -> ConnectFuture {
        let addr = format!("{}:993", server);
        let addr = addr.to_socket_addrs().unwrap().next().unwrap();
        let stream = TcpStream::connect(&addr, handle);
        ConnectFuture::TcpConnecting(stream, server.to_string())
    }

    fn call(self, cmd: proto::Command) -> CommandFuture {
        let Client { transport, mut state } = self;
        let request_id = proto::tag(state.next_request_id);
        state.next_request_id += 1;
        let future = transport.send(proto::Request(request_id.clone(), cmd));
        CommandFuture {
            future: Some(future),
            transport: None,
            state: Some(state),
            request_id: request_id,
            next_state: None,
            responses: Some(ServerMessages::new()),
        }
    }

    pub fn login(self, account: &str, password: &str) -> CommandFuture {
        let cmd = proto::Command::Login(account.to_string(), password.to_string());
        let mut future = self.call(cmd);
        future.next_state = Some(proto::State::Authenticated);
        future
    }

    pub fn select(self, mailbox: &str) -> CommandFuture {
        let cmd = proto::Command::Select(mailbox.to_string());
        let mut future = self.call(cmd);
        future.next_state = Some(proto::State::Selected);
        future
    }
}
