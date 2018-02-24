use bytes::{BufMut, Bytes, BytesMut};

use futures;

use nom::{IResult, Needed};

use imap_proto;
use imap_proto::types::{Request, RequestId, Response};

use std::io;
use std::mem;

use tokio::net::TcpStream;
use tokio_io::codec::{Decoder, Encoder, Framed};
use tokio_tls::TlsStream;

pub struct ImapCodec {
    decode_need_message_bytes: usize,
}

impl Default for ImapCodec {
    fn default() -> Self {
        Self {
            decode_need_message_bytes: 0,
        }
    }
}

impl<'a> Decoder for ImapCodec {
    type Item = ResponseData;
    type Error = io::Error;
    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, io::Error> {
        if self.decode_need_message_bytes > buf.len() {
            return Ok(None);
        }
        let (response, rsp_len) = match imap_proto::parse_response(buf) {
            IResult::Done(remaining, response) => {
                // This SHOULD be acceptable/safe: BytesMut storage memory is
                // allocated on the heap and should not move. It will not be
                // freed as long as we keep a reference alive, which we do
                // by retaining a reference to the split buffer, below.
                let response = unsafe { mem::transmute(response) };
                (response, buf.len() - remaining.len())
            },
            IResult::Incomplete(Needed::Size(min)) => {
                self.decode_need_message_bytes = min;
                return Ok(None);
            },
            IResult::Incomplete(_) => {
                return Ok(None);
            },
            IResult::Error(err) => {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("{} during parsing of {:?}", err, buf),
                ));
            },
        };
        let raw = buf.split_to(rsp_len).freeze();
        self.decode_need_message_bytes = 0;
        Ok(Some(ResponseData { raw, response }))
    }
}

impl Encoder for ImapCodec {
    type Item = Request;
    type Error = io::Error;
    fn encode(&mut self, msg: Self::Item, dst: &mut BytesMut) -> Result<(), io::Error> {
        dst.put(msg.0.as_bytes());
        dst.put(b' ');
        dst.put(&msg.1);
        dst.put("\r\n");
        Ok(())
    }
}

#[derive(Debug)]
pub struct ResponseData {
    raw: Bytes,
    // This reference is really scoped to the lifetime of the `raw`
    // member, but unfortunately Rust does not allow that yet. It
    // is transmuted to `'static` by the `Decoder`, instead, and
    // references returned to callers of `ResponseData` are limited
    // to the lifetime of the `ResponseData` struct.
    pub response: Response<'static>,
}

impl ResponseData {
    pub fn request_id(&self) -> Option<&RequestId> {
        match self.response {
            Response::Done { ref tag, .. } => Some(tag),
            _ => None,
        }
    }
    pub fn parsed(&self) -> &Response {
        unsafe { mem::transmute(&self.response) }
    }
}

pub type ImapTls = Framed<TlsStream<TcpStream>, ImapCodec>;

impl ImapTransport for ImapTls {}

pub trait ImapTransport
    : futures::Stream<Item = ResponseData, Error = io::Error>
    + futures::Sink<SinkItem = Request, SinkError = io::Error> {
}
