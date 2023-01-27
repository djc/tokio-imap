use std::borrow::Cow;
use std::convert::TryInto;
use std::io;
use std::net::ToSocketAddrs;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_sink::Sink;
use futures_util::{ready, stream::Stream, StreamExt};
use pin_project::pin_project;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_rustls::rustls::{ClientConfig, OwnedTrustAnchor};
use tokio_rustls::{client::TlsStream, TlsConnector};
use tokio_util::codec::{Decoder, Framed};

use crate::codec::{ImapCodec, ResponseData};
use imap_proto::builders::command::Command;
use imap_proto::{Request, RequestId, State};

pub type TlsClient = Client<TlsStream<TcpStream>>;

pub struct Client<T> {
    transport: Framed<T, ImapCodec>,
    state: State,
    request_ids: IdGenerator,
}

impl TlsClient {
    pub async fn connect(server: &str) -> io::Result<(ResponseData, Self)> {
        let addr = (server, 993).to_socket_addrs()?.next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("no IP addresses found for {server}"),
            )
        })?;

        let mut roots = tokio_rustls::rustls::RootCertStore::empty();
        roots.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(|ta| {
            OwnedTrustAnchor::from_subject_spki_name_constraints(
                ta.subject,
                ta.spki,
                ta.name_constraints,
            )
        }));

        let connector = TlsConnector::from(Arc::new(
            ClientConfig::builder()
                .with_safe_defaults()
                .with_root_certificates(roots)
                .with_no_client_auth(),
        ));

        let stream = TcpStream::connect(&addr).await?;
        let stream = connector
            .connect(server.try_into().unwrap(), stream)
            .await?;
        let mut transport = ImapCodec::default().framed(stream);

        let greeting = match transport.next().await {
            Some(greeting) => Ok(greeting),
            None => Err(io::Error::new(io::ErrorKind::Other, "no greeting found")),
        }?;
        let client = Client {
            transport,
            state: State::NotAuthenticated,
            request_ids: IdGenerator::new(),
        };

        greeting.map(|greeting| (greeting, client))
    }

    pub fn call<C: Into<Command>>(&mut self, cmd: C) -> ResponseStream<TlsStream<TcpStream>> {
        let request_id = self.request_ids.next().unwrap(); // safe: never returns Err,
        ResponseStream {
            client: self,
            request_id,
            cmd: cmd.into(),
            state: ResponseStreamState::Start,
        }
    }
}

#[pin_project]
pub struct ResponseStream<'a, T> {
    #[pin]
    client: &'a mut Client<T>,
    request_id: RequestId,
    cmd: Command,
    state: ResponseStreamState,
}

impl<'a, T> Stream for ResponseStream<'a, T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    type Item = Result<ResponseData, io::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut me = self.project();
        loop {
            match me.state {
                ResponseStreamState::Start => {
                    ready!(Pin::new(&mut me.client.transport).poll_ready(cx))?;
                    let pinned = Pin::new(&mut me.client.transport);
                    pinned.start_send(&Request(
                        Cow::Borrowed(me.request_id.as_bytes()),
                        Cow::Borrowed(&me.cmd.args),
                    ))?;
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

                            if let Some(next_state) = me.cmd.next_state.as_ref() {
                                me.client.state = *next_state;
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
