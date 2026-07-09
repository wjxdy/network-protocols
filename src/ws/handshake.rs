use crate::http::Request;

use super::crypto::{base64_encode, sha1};

const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

pub fn websocket_accept_key(key: &str) -> String {
    let combined = format!("{}{}", key.trim(), WS_GUID);

    let digest = sha1(combined.as_bytes());

    base64_encode(&digest)
}

pub fn is_websocket_upgrade(request: &Request) -> bool {
    if request.method != "GET" {
        return false;
    }

    let Some(upgrade) = request.header("Upgrade") else {
        return false;
    };

    if !upgrade.eq_ignore_ascii_case("websocket") {
        return false;
    }

    request.header("Sec-WebSocket-key").is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_accept_key() {
        let key = "dGhlIHNhbXBsZSBub25jZQ==";
        assert_eq!(websocket_accept_key(key), "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }
}
