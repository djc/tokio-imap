use std::future::Future;
use std::io;
use std::net::ToSocketAddrs;
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use tokio::codec::Decoder;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::tcp::TcpStream;
use tokio_rustls::rustls::ClientConfig;
use tokio_rustls::webpki::DNSNameRef;
use tokio_rustls::{client::TlsStream, TlsConnector};

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

    pub fn call(&mut self, cmd: Command) -> ResponseStream<TlsStream<TcpStream>> {
        ResponseStream::new(self, cmd)
    }
}

pub struct ResponseStream<'a, T> {
    client: &'a mut Client<T>,
    request: Request,
    next_state: Option<State>,
    sending: bool,
    done: bool,
}

impl<'a, T> ResponseStream<'a, T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(client: &mut Client<T>, cmd: Command) -> ResponseStream<'_, T> {
        let request_id = client.state.request_ids.next().unwrap(); // safe: never returns Err
        let (cmd_bytes, next_state) = cmd.into_parts();
        let request = Request(request_id.clone(), cmd_bytes);

        ResponseStream {
            client,
            request,
            next_state,
            sending: true,
            done: false,
        }
    }

    pub async fn next(&mut self) -> Option<Result<ResponseData, io::Error>> {
        if self.done {
            return None;
        }

        if self.sending {
            match self.client.transport.send(self.request.clone()).await {
                Ok(()) => {
                    self.sending = false;
                }
                Err(e) => return Some(Err(e)),
            }
        }

        match self.client.transport.next().await {
            Some(Ok(rsp)) => {
                if let Some(req_id) = rsp.request_id() {
                    self.done = *req_id == self.request.0;
                }

                if self.done {
                    if let Some(next_state) = self.next_state.take() {
                        self.client.state.state = next_state;
                    }
                }

                Some(Ok(rsp))
            }
            Some(Err(e)) => Some(Err(e)),
            None => Some(Err(io::Error::new(
                io::ErrorKind::Other,
                "stream ended before command completion",
            ))),
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
