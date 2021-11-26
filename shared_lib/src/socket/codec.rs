use actix_codec::{Decoder, Encoder};
use bytes::{Buf, BufMut, BytesMut};

use crate::socket::message::{BotResponse, ServerRequest};

/// Codec for Client -> Server transport
pub struct ClientCodec;

impl Decoder for ClientCodec {
    type Item = ServerRequest;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let size = {
            if src.len() < 2 {
                return Ok(None);
            }
            src.get_u16() as usize
        };

        if src.len() >= size {
            let buf = src.split_to(size);
            Ok(Some(serde_json::from_slice::<Self::Item>(&buf)?))
        } else {
            Ok(None)
        }
    }
}

impl Encoder<BotResponse> for ClientCodec {
    type Error = std::io::Error;

    fn encode(&mut self, msg: BotResponse, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let msg = serde_json::to_string(&msg).unwrap();
        let msg_ref: &[u8] = msg.as_ref();

        dst.reserve(msg_ref.len() + 2);
        dst.put_u16(msg_ref.len() as u16);
        dst.put(msg_ref);

        Ok(())
    }
}

/// Codec for Server -> Client transport
pub struct ServerCodec;

impl Decoder for ServerCodec {
    type Item = BotResponse;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let size = {
            if src.len() < 2 {
                return Ok(None);
            }
            src.get_u16() as usize
        };

        if src.len() >= size {
            let buf = src.split_to(size);
            Ok(Some(serde_json::from_slice::<BotResponse>(&buf)?))
        } else {
            Ok(None)
        }
    }
}

impl Encoder<ServerRequest> for ServerCodec {
    type Error = std::io::Error;

    fn encode(&mut self, msg: ServerRequest, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let msg = serde_json::to_string(&msg).unwrap();
        let msg_ref: &[u8] = msg.as_ref();

        dst.reserve(msg_ref.len() + 2);
        dst.put_u16(msg_ref.len() as u16);
        dst.put(msg_ref);

        Ok(())
    }
}
