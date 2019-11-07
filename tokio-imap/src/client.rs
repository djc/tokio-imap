use std::io;
use std::net::ToSocketAddrs;

use futures::sink::Send;
use futures::stream::Stream;
use futures::{Async, Future, Poll, Sink};
use futures_state_stream::{StateStream, StreamEvent};
use native_tls::TlsConnector;
use tokio::codec::Decoder;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::tcp::{ConnectFuture, TcpStream};
use tokio_tls::{self, Connect, TlsStream};

use crate::proto::{ImapCodec, ImapTransport, ResponseData};
use imap_proto::builders::command::Command;
use imap_proto::{Request, RequestId, State};

pub mod builder {
    pub use imap_proto::builders::command::{
        CommandBuilder, FetchBuilderAttributes, FetchBuilderMessages, FetchBuilderModifiers,
        FetchCommand, FetchCommandAttributes, FetchCommandMessages,
    };
}

pub type TlsClient = Client<TlsStream<TcpStream>>;

pub struct Client<T> {
    transport: ImapTransport<T>,
    state: ClientState,
}

impl<T> Client<T>
where
    T: AsyncRead + AsyncWrite,
{
    pub fn call(self, cmd: Command) -> ResponseStream<T> {
        let Client {
            transport,
            mut state,
        } = self;
        let request_id = state.request_ids.next().unwrap(); // safe: never returns Err
        let (cmd_bytes, next_state) = cmd.into_parts();
        let future = transport.send(Request(request_id.clone(), cmd_bytes));
        ResponseStream::new(future, state, request_id, next_state)
    }
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

pub struct ResponseStream<T>
where
    T: AsyncRead + AsyncWrite,
{
    future: Option<Send<ImapTransport<T>>>,
    transport: Option<ImapTransport<T>>,
    state: Option<ClientState>,
    request_id: RequestId,
    next_state: Option<State>,
    done: bool,
}

impl<T> ResponseStream<T>
where
    T: AsyncRead + AsyncWrite,
{
    pub fn new(
        future: Send<ImapTransport<T>>,
        state: ClientState,
        request_id: RequestId,
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

impl<T> StateStream for ResponseStream<T>
where
    T: AsyncRead + AsyncWrite,
{
    type Item = ResponseData;
    type State = Client<T>;
    type Error = io::Error;
    fn poll(&mut self) -> Poll<StreamEvent<Self::Item, Self::State>, Self::Error> {
        if let Some(mut future) = self.future.take() {
            match future.poll() {
                Ok(Async::Ready(transport)) => {
                    self.transport = Some(transport);
                }
                Ok(Async::NotReady) => {
                    self.future = Some(future);
                    return Ok(Async::NotReady);
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
        let mut transport = match self.transport.take() {
            None => return Ok(Async::NotReady),
            Some(transport) => transport,
        };
        if self.done {
            let mut state = self.state.take().unwrap(); // safe: initialized from start
            if let Some(next_state) = self.next_state.take() {
                state.state = next_state;
            }
            return Ok(Async::Ready(StreamEvent::Done(Client { transport, state })));
        }
        match transport.poll() {
            Ok(Async::Ready(Some(rsp))) => {
                if let Some(req_id) = rsp.request_id() {
                    self.done = *req_id == self.request_id;
                };
                self.transport = Some(transport);
                return Ok(Async::Ready(StreamEvent::Next(rsp)));
            }
            Err(e) => {
                return Err(e);
            }
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
    ServerGreeting(Option<ImapTransport<TlsStream<TcpStream>>>),
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
            let transport = ImapCodec::default().framed(try_ready!(future
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
                .poll()));
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
