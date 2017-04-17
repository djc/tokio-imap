use bytes::{BufMut, BytesMut};

use std::io;
use std::str;

use tokio_core::net::TcpStream;
use tokio_io::codec::{Decoder, Encoder, Framed};
use tokio_tls::TlsStream;

pub type ImapTransport = Framed<TlsStream<TcpStream>, ImapCodec>;

pub enum State {
    NotAuthenticated,
    Authenticated,
    Selected,
    Logout,
}

pub struct ImapCodec;

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

