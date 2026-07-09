use std::io::{Read, Write};
use std::net::TcpStream;

#[derive(Debug)]
pub enum WsFrame {
    Text(String),
    Close,
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Other,
}

pub fn read_ws_frame(stream: &mut TcpStream) -> std::io::Result<WsFrame> {
    let mut header = [0u8; 2];
    stream.read_exact(&mut header)?;

    let byte0 = header[0];

    let byte1 = header[1];

    let fin = byte0 & 0b1000_0000 != 0;

    let opcode = byte0 & 0b0000_1111;

    let masked = byte1 & 0b1000_0000 != 0;

    let mut payload_len = (byte1 & 0b0111_1111) as u64;

    if !fin {
        return Ok(WsFrame::Other);
    }

    if payload_len == 126 {
        let mut extended = [0u8; 2];
        stream.read_exact(&mut extended)?;
        payload_len = u16::from_be_bytes(extended) as u64;
    }

    if !masked {
        return Ok(WsFrame::Other);
    }

    let mut mask_key = [0u8; 4];
    stream.read_exact(&mut mask_key)?;

    let mut payload = vec![0u8; payload_len as usize];
    stream.read_exact(&mut payload)?;

    for i in 0..payload.len() {
        payload[i] ^= mask_key[i % 4];
    }

    match opcode {
        0x1 => {
            let text = String::from_utf8_lossy(&payload).to_string();
            Ok(WsFrame::Text(text))
        }

        0x8 => Ok(WsFrame::Close),
        0x9 => Ok(WsFrame::Ping(payload)),
        0xA => Ok(WsFrame::Pong(payload)),
        _ => Ok(WsFrame::Other),
    }
}

pub fn write_ws_text(stream: &mut TcpStream, text: &str) -> std::io::Result<()> {
    let payload = text.as_bytes();

    let mut frame = Vec::new();

    frame.push(0x81);

    if payload.len() <= 125 {
        frame.push(payload.len() as u8);
    } else if payload.len() <= 65535 {
        frame.push(126);
        frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    } else {
        frame.push(127);
        frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    }

    frame.extend_from_slice(payload);

    stream.write_all(&frame)
}
