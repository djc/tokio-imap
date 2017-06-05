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

use proto::*;

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

    fn call(self, cmd: Command) -> CommandFuture {
        let Client { transport, mut state } = self;
        let request_id = state.request_ids.next().unwrap();
        let future = transport.send(Request(request_id.clone(), cmd));
        CommandFuture::new(future, state, request_id)
    }

    pub fn login(self, account: &str, password: &str) -> CommandFuture {
        let cmd = Command::Login(account.to_string(), password.to_string());
        let mut future = self.call(cmd);
        future.next_state = Some(State::Authenticated);
        future
    }

    pub fn select(self, mailbox: &str) -> CommandFuture {
        let cmd = Command::Select(mailbox.to_string());
        let mut future = self.call(cmd);
        future.next_state = Some(State::Selected);
        future
    }
}

pub struct CommandFuture {
    future: Option<Send<ImapTransport>>,
    transport: Option<ImapTransport>,
    state: Option<ClientState>,
    request_id: RequestId,
    next_state: Option<State>,
    responses: Option<ServerMessages>,
}

impl CommandFuture {
    pub fn new(future: Send<ImapTransport>, state: ClientState,
               request_id: RequestId) -> CommandFuture {
        CommandFuture {
            future: Some(future),
            transport: None,
            state: Some(state),
            request_id: request_id,
            next_state: None,
            responses: Some(ServerMessages::new()),
        }
    }

    fn push_frame(&mut self, frame: Response) {
        match self.responses {
            Some(ref mut responses) => {
                responses.frames.push(frame);
            },
            None => panic!("unpossible"),
        }
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
                Ok(Async::Ready(Some(Response::Status(Some(req_id), msg)))) => {
                    let expected = req_id == self.request_id;
                    let rsp = Response::Status(Some(req_id), msg);
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

pub enum ConnectFuture {
    #[doc(hidden)]
    TcpConnecting(TcpStreamNew, String),
    #[doc(hidden)]
    TlsHandshake(ConnectAsync<TcpStream>),
    #[doc(hidden)]
    ServerGreeting(Option<ImapTransport>),
}

impl Future for ConnectFuture {
    type Item = (Client, Response);
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
            }).poll()).framed(ImapCodec);
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
    pub fn new() -> ClientState {
        ClientState {
            state: State::NotAuthenticated,
            request_ids: IdGenerator::new(),
        }
    }
}

impl ServerMessages {
    fn new() -> ServerMessages {
        ServerMessages { frames: Vec::new() }
    }
}

pub struct ServerMessages {
    pub frames: Vec<Response>,
}
