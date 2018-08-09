use futures::{Async, Future, Poll, Sink};
use futures::stream::Stream;
use futures::sink::Send;
use futures_state_stream::{StateStream, StreamEvent};

use native_tls::TlsConnector;

use std::io;
use std::net::ToSocketAddrs;

use tokio::net::{ConnectFuture, TcpStream};
use tokio_codec::Decoder;
use tokio_tls::{self, Connect};

use imap_proto::{Request, RequestId, State};
use imap_proto::builders::command::Command;
use proto::{ImapCodec, ImapTls, ImapTransport, ResponseData};

pub mod builder {
    pub use imap_proto::builders::command::{CommandBuilder, FetchBuilderAttributes,
                                            FetchBuilderMessages, FetchBuilderModifiers,
                                            FetchCommand, FetchCommandAttributes,
                                            FetchCommandMessages};
}

pub trait ImapClient {
    type Transport: ImapTransport;
    fn into_parts(self) -> (Self::Transport, ClientState);
    fn rebuild(transport: Self::Transport, state: ClientState) -> Self;

    fn call(self, cmd: Command) -> ResponseStream<Self>
    where
        Self: ImapClient + Sized, {
        let (transport, mut state) = self.into_parts();
        let request_id = state.request_ids.next().unwrap(); // safe: never returns Err
        let (cmd_bytes, next_state) = cmd.into_parts();
        let future = transport.send(Request(request_id.clone(), cmd_bytes));
        ResponseStream::new(future, state, request_id, next_state)
    }
}

pub struct TlsClient {
    transport: ImapTls,
    state: ClientState,
}

impl TlsClient {
    pub fn connect(server: &str) -> io::Result<ImapConnectFuture> {
        let addr = (server, 993).to_socket_addrs()?.next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("no IP addresses found for {}", server),
            )
        })?;
        Ok(ImapConnectFuture::TcpConnecting(
            TcpStream::connect(&addr),
            server.to_string(),
        ))
    }
}

impl ImapClient for TlsClient {
    type Transport = ImapTls;

    fn into_parts(self) -> (ImapTls, ClientState) {
        let Self { transport, state } = self;
        (transport, state)
    }

    fn rebuild(transport: ImapTls, state: ClientState) -> TlsClient {
        TlsClient { transport, state }
    }
}

pub struct ResponseStream<E>
where
    E: ImapClient, {
    future: Option<Send<E::Transport>>,
    transport: Option<E::Transport>,
    state: Option<ClientState>,
    request_id: RequestId,
    next_state: Option<State>,
    done: bool,
}

impl<E> ResponseStream<E>
where
    E: ImapClient,
{
    pub fn new(
        future: Send<E::Transport>, state: ClientState, request_id: RequestId,
        next_state: Option<State>,
    ) -> Self {
        Self {
            future: Some(future),
            transport: None,
            state: Some(state),
            request_id,
            next_state,
            done: false,
        }
    }
}

impl<E> StateStream for ResponseStream<E>
where
    E: ImapClient,
{
    type Item = ResponseData;
    type State = E;
    type Error = io::Error;
    fn poll(&mut self) -> Poll<StreamEvent<Self::Item, Self::State>, Self::Error> {
        if let Some(mut future) = self.future.take() {
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
        let mut transport = match self.transport.take() {
            None => return Ok(Async::NotReady),
            Some(mut transport) => transport,
        };
        if self.done {
            let mut state = self.state.take().unwrap(); // safe: initialized from start
            if let Some(next_state) = self.next_state.take() {
                state.state = next_state;
            }
            return Ok(Async::Ready(StreamEvent::Done(E::rebuild(
                transport,
                state,
            ))));
        }
        match transport.poll() {
            Ok(Async::Ready(Some(rsp))) => {
                if let Some(req_id) = rsp.request_id() {
                    self.done = *req_id == self.request_id;
                };
                self.transport = Some(transport);
                return Ok(Async::Ready(StreamEvent::Next(rsp)));
            },
            Err(e) => {
                return Err(e);
            },
            _ => (),
        }
        self.transport = Some(transport);
        Ok(Async::NotReady)
    }
}

pub enum ImapConnectFuture {
    #[doc(hidden)]
    TcpConnecting(ConnectFuture, String),
    #[doc(hidden)]
    TlsHandshake(Connect<TcpStream>),
    #[doc(hidden)]
    ServerGreeting(Option<ImapTls>),
}

impl Future for ImapConnectFuture {
    type Item = (ResponseData, TlsClient);
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut new = None;
        if let ImapConnectFuture::TcpConnecting(ref mut future, ref domain) = *self {
            let stream = try_ready!(future.poll());
            let ctx = TlsConnector::builder().build().unwrap();
            let ctx = tokio_tls::TlsConnector::from(ctx);
            new = Some(ImapConnectFuture::TlsHandshake(ctx.connect(domain, stream)));
        }
        if new.is_some() {
            *self = new.take().unwrap();
        }
        if let ImapConnectFuture::TlsHandshake(ref mut future) = *self {
            let transport = ImapCodec::default().framed(try_ready!(
                future
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
                    .poll()
            ));
            new = Some(ImapConnectFuture::ServerGreeting(Some(transport)));
        }
        if new.is_some() {
            *self = new.take().unwrap();
        }
        if let ImapConnectFuture::ServerGreeting(ref mut wrapped) = *self {
            let msg = try_ready!(wrapped.as_mut().unwrap().poll()).unwrap();
            return Ok(Async::Ready((
                msg,
                TlsClient {
                    transport: wrapped.take().unwrap(),
                    state: ClientState::new(),
                },
            )));
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
    fn default() -> Self {
        Self::new()
    }
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
    fn default() -> Self {
        Self::new()
    }
}

impl Iterator for IdGenerator {
    type Item = RequestId;
    fn next(&mut self) -> Option<Self::Item> {
        self.next += 1;
        Some(RequestId(format!("A{:04}", self.next % 10_000)))
    }
}
