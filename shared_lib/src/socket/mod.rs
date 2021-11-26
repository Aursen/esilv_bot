pub mod client;
pub mod codec;
pub mod message;
pub mod server;
pub mod session;

#[cfg(test)]
mod tests {
    use actix_codec::{Decoder, Encoder};
    use bytes::{BufMut, BytesMut};
    use serde_json::json;

    use crate::socket::{codec::ServerCodec, message::ServerRequest};

    use super::{codec::ClientCodec, message::BotResponse};

    #[test]
    fn client_codec_encode() {
        let mut codec = ClientCodec;
        let user = json!( {
            "discord_id": 123
        });
        let user_msg = BotResponse::User(serde_json::to_string(&user).unwrap());
        let mut bytes = BytesMut::new();

        assert!(codec.encode(user_msg, &mut bytes,).is_ok());
        assert!(codec.encode(BotResponse::Ping, &mut bytes).is_ok());
    }

    #[test]
    fn client_codec_decode() {
        let mut codec = ClientCodec;
        let content = b"\0\"{\"GetUser\":\"{\\\"discord_id\\\":123}\"}\0\x06\"Ping\"";

        let mut bytes = BytesMut::new();
        bytes.reserve(content.len());
        bytes.put(&content[..]);

        let user_result = codec.decode(&mut bytes).unwrap();
        let ping_result = codec.decode(&mut bytes).unwrap();

        assert!(matches!(user_result, Some(ServerRequest::GetUser(u)) if u.contains("discord_id")));
        assert!(matches!(ping_result, Some(ServerRequest::Ping)));
    }

    #[test]
    fn server_codec_encode() {
        let mut codec = ServerCodec;
        let user = json!( {
            "discord_id": 123
        });
        let user_msg = ServerRequest::GetUser(serde_json::to_string(&user).unwrap());
        let mut bytes = BytesMut::new();

        assert!(codec.encode(user_msg, &mut bytes,).is_ok());
        assert!(codec.encode(ServerRequest::Ping, &mut bytes).is_ok());
    }

    #[test]
    fn server_codec_decode() {
        let mut codec = ServerCodec;
        let content = b"\0\x1f{\"User\":\"{\\\"discord_id\\\":123}\"}\0\x06\"Ping\"";
        let mut bytes = BytesMut::new();
        bytes.reserve(content.len());
        bytes.put(&content[..]);

        let user_result = codec.decode(&mut bytes).unwrap();
        let ping_result = codec.decode(&mut bytes).unwrap();

        assert!(matches!(user_result, Some(BotResponse::User(u)) if u.contains("discord_id")));
        assert!(matches!(ping_result, Some(BotResponse::Ping)));
    }
}
