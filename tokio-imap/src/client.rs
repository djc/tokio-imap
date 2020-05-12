use std::future::Future;
use std::io;
use std::net::ToSocketAddrs;
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
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

pub struct ResponseStream<'a, C, T> {
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
            state: ResponseStreamState::Sending,
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub async fn next(&mut self) -> Option<Result<ResponseData, io::Error>> {
        loop {
            match &mut self.state {
                ResponseStreamState::Sending => {
                    let request = Request(self.request_id.as_ref(), self.cmd.command_bytes());
                    match self.client.transport.send(&request).await {
                        Ok(()) => {
                            self.state = ResponseStreamState::Receiving;
                        }
                        Err(e) => return Some(Err(e)),
                    }
                }
                ResponseStreamState::Receiving => match self.client.transport.next().await {
                    Some(Ok(rsp)) => {
                        match rsp.request_id() {
                            Some(req_id) if req_id == &self.request_id => {}
                            Some(_) | None => return Some(Ok(rsp)),
                        }

                        if let Some(next_state) = self.cmd.next_state() {
                            self.client.state.state = next_state;
                        }
                        self.state = ResponseStreamState::Done;
                        return Some(Ok(rsp));
                    }
                    Some(Err(e)) => return Some(Err(e)),
                    None => {
                        return Some(Err(io::Error::new(
                            io::ErrorKind::Other,
                            "stream ended before command completion",
                        )))
                    }
                },
                ResponseStreamState::Done => {
                    return None;
                }
            }
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

enum ResponseStreamState {
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
