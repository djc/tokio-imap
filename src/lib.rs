extern crate bytes;
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
use futures::Future;
use futures::future::ok;
use futures::stream::{SplitSink, SplitStream, Stream};
use native_tls::{TlsConnector};
use tokio_core::net::TcpStream;
use tokio_io::AsyncRead;
use tokio_io::codec::{Decoder, Encoder, Framed};
use tokio_core::reactor::Core;
use tokio_tls::{TlsConnectorExt, TlsStream};

struct ImapCodec;

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

pub struct Client {
    core: Core,
    sink: SplitSink<Framed<TlsStream<TcpStream>, ImapCodec>>,
    src: SplitStream<Framed<TlsStream<TcpStream>, ImapCodec>>,
}

impl Client {
    pub fn connect(server: &str) -> Client {
        let mut core = Core::new().unwrap();
        let handle = core.handle();
        let addr = format!("{}:993", server);
        let addr = addr.to_socket_addrs().unwrap().next().unwrap();

        let cx = TlsConnector::builder().unwrap().build().unwrap();
        let socket = TcpStream::connect(&addr, &handle);
        let events = socket.and_then(|socket| {
            let tls = cx.connect_async(server, socket);
            tls.map_err(|e| {
                io::Error::new(io::ErrorKind::Other, e)
            })
        }).and_then(|stream| {
            ok(stream.framed(ImapCodec).split())
        });
        let (sink, src) = core.run(events).unwrap();
        Client { core: core, sink: sink, src: src }
    }

    pub fn login(mut self, account: &str, password: &str) {
        let res = self.src.filter_map(|data| {
            println!("{}", data);
            Some(format!("a001 LOGIN {} {}", account, password))
        }).forward(self.sink);
        self.core.run(res).unwrap();
    }
}
