#[derive(Debug)]
pub enum WsFrame {
    Text(String),
    Close,
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Other,
}
