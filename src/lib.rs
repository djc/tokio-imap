extern crate futures;
extern crate native_tls;
extern crate tokio_core;
extern crate tokio_tls;
extern crate tokio_proto;

use std::io;
use std::net::ToSocketAddrs;
use std::str;

use futures::Future;
use futures::future::ok;
use futures::stream::{SplitSink, SplitStream, Stream};
use native_tls::{TlsConnector};
use tokio_core::io::{Codec, EasyBuf, Framed, Io};
use tokio_core::net::TcpStream;
use tokio_core::reactor::Core;
use tokio_tls::{TlsConnectorExt, TlsStream};

struct ImapCodec;

impl Codec for ImapCodec {
    type In = String;
    type Out = String;

    fn decode(&mut self, buf: &mut EasyBuf)
             -> Result<Option<String>, io::Error> {
        if let Some(n) = buf.as_slice().iter().position(|b| *b == b'\n') {
            // remove the serialized frame from the buffer.
            let line = buf.drain_to(n);

            // Also remove the '\n'
            buf.drain_to(1);

            // Turn this data into a UTF string and return it in a Frame.
            return match str::from_utf8(line.as_slice()) {
                Ok(s) => Ok(Some(s.to_string())),
                Err(_) => Err(io::Error::new(io::ErrorKind::Other,
                                             "invalid string")),
            }
        } else {
            Ok(None)
        }
    }

    fn encode(&mut self, msg: String, buf: &mut Vec<u8>) -> io::Result<()> {
        buf.extend(msg.as_bytes());
        buf.extend(&[b'\r', b'\n']);
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
