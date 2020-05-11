use std::future::Future;
use std::io;
use std::net::ToSocketAddrs;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::{ready, Sink, Stream, StreamExt};
use pin_project::{pin_project, project};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_rustls::rustls::ClientConfig;
use tokio_rustls::webpki::DNSNameRef;
use tokio_rustls::{client::TlsStream, TlsConnector};
use tokio_util::codec::Decoder;

use crate::proto::{ImapCodec, ImapTransport, ResponseData};
use imap_proto::builders::command::CommandBytes;
use imap_proto::{Request, RequestId, State};

pub mod builder {
    pub use imap_proto::builders::command::{fetch, CommandBuilder, CommandBytes};
}

pub type TlsClient = Client<TlsStream<TcpStream>>;

pub struct Client<T> {
    transport: ImapTransport<T>,
    state: ClientState,
}

impl TlsClient {
    pub async fn connect(server: &str) -> io::Result<(ResponseData, Self)> {
        let addr = (server, 993).to_socket_addrs()?.next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("no IP addresses found for {}", server),
            )
        })?;

        let mut tls_config = ClientConfig::new();
        tls_config
            .root_store
            .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
        let connector: TlsConnector = Arc::new(tls_config).into();

        let stream = TcpStream::connect(&addr).await?;
        let stream = connector
            .connect(DNSNameRef::try_from_ascii_str(server).unwrap(), stream)
            .await?;
        let mut transport = ImapCodec::default().framed(stream);

        let greeting = match transport.next().await {
            Some(greeting) => Ok(greeting),
            None => Err(io::Error::new(io::ErrorKind::Other, "no greeting found")),
        }?;
        let client = Client {
            transport,
            state: ClientState::new(),
        };

        greeting.map(|greeting| (greeting, client))
    }

    pub fn call<'a, C: CommandBytes>(
        &'a mut self,
        cmd: &'a C,
    ) -> ResponseStream<'a, C, TlsStream<TcpStream>> {
        ResponseStream::new(self, cmd)
    }
}

#[pin_project]
pub struct ResponseStream<'a, C, T> {
    #[pin]
    client: &'a mut Client<T>,
    request_id: RequestId,
    cmd: &'a C,
    state: ResponseStreamState,
}

impl<'a, C, T> ResponseStream<'a, C, T>
where
    C: CommandBytes,
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new<'n>(client: &'n mut Client<T>, cmd: &'n C) -> ResponseStream<'n, C, T> {
        let request_id = client.state.request_ids.next().unwrap(); // safe: never returns Err
        ResponseStream {
            client,
            request_id,
            cmd,
            state: ResponseStreamState::Start,
        }
    }

    pub async fn try_collect(&mut self) -> Result<Vec<ResponseData>, io::Error> {
        let mut data = vec![];
        loop {
            match self.next().await {
                Some(Ok(rsp)) => {
                    data.push(rsp);
                }
                Some(Err(e)) => return Err(e),
                None => return Ok(data),
            }
        }
    }

    pub async fn try_for_each<F, Fut>(&mut self, mut f: F) -> Result<(), io::Error>
    where
        F: FnMut(ResponseData) -> Fut,
        Fut: Future<Output = Result<(), io::Error>>,
    {
        loop {
            match self.next().await {
                Some(Ok(rsp)) => f(rsp).await?,
                Some(Err(e)) => return Err(e),
                None => return Ok(()),
            }
        }
    }

    pub async fn try_fold<S, Fut, F>(&mut self, mut state: S, mut f: F) -> Result<S, io::Error>
    where
        F: FnMut(S, ResponseData) -> Fut,
        Fut: Future<Output = Result<S, io::Error>>,
    {
        loop {
            match self.next().await {
                Some(Ok(rsp)) => match f(state, rsp).await {
                    Ok(new) => {
                        state = new;
                    }
                    Err(e) => return Err(e),
                },
                Some(Err(e)) => return Err(e),
                None => return Ok(state),
            }
        }
    }
}

impl<'a, C, T> Stream for ResponseStream<'a, C, T>
where
    C: CommandBytes,
    T: AsyncRead + AsyncWrite + Unpin,
{
    type Item = Result<ResponseData, io::Error>;

    #[project]
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut me = self.project();
        loop {
            match me.state {
                ResponseStreamState::Start => {
                    ready!(Pin::new(&mut me.client.transport).poll_ready(cx))?;
                    let pinned = Pin::new(&mut me.client.transport);
                    pinned.start_send(&Request(me.request_id.as_ref(), me.cmd.command_bytes()))?;
                    *me.state = ResponseStreamState::Sending;
                }
                ResponseStreamState::Sending => {
                    let pinned = Pin::new(&mut me.client.transport);
                    ready!(pinned.poll_flush(cx))?;
                    *me.state = ResponseStreamState::Receiving;
                }
                ResponseStreamState::Receiving => {
                    match ready!(Pin::new(&mut me.client.transport).poll_next(cx)) {
                        Some(Ok(rsp)) => {
                            match rsp.request_id() {
                                Some(req_id) if req_id == me.request_id => {}
                                Some(_) | None => return Poll::Ready(Some(Ok(rsp))),
                            }

                            if let Some(next_state) = me.cmd.next_state() {
                                me.client.state.state = next_state;
                            }
                            *me.state = ResponseStreamState::Done;
                            return Poll::Ready(Some(Ok(rsp)));
                        }
                        Some(Err(e)) => return Poll::Ready(Some(Err(e))),
                        None => {
                            return Poll::Ready(Some(Err(io::Error::new(
                                io::ErrorKind::Other,
                                "stream ended before command completion",
                            ))))
                        }
                    }
                }
                ResponseStreamState::Done => {
                    return Poll::Ready(None);
                }
            }
        }
    }
}

enum ResponseStreamState {
    Start,
    Sending,
    Receiving,
    Done,
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
