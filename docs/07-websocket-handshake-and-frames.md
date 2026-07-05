# 第 6 阶段：实现 WebSocket 握手和 Frame

这一阶段开始实现 WebSocket。

WebSocket 比 SSE 更复杂，因为它不是一直使用普通 HTTP body。

它的流程是：

```text
TCP 连接
  ↓
HTTP Upgrade 握手
  ↓
切换成 WebSocket frame 协议
  ↓
客户端和服务端双向发送消息
```

这也是你第一次在项目里遇到：

```text
先用文本协议握手
再切换成二进制协议通信
```

## 学习优先的实现策略

WebSocket 握手需要：

```text
Sec-WebSocket-Key
Sec-WebSocket-Accept
SHA-1
Base64
```

如果用库，几行就能做完。

但这个项目是为了学习底层协议，所以这一阶段我们选择：

```text
自己写一个最小 Base64
自己写一个最小 SHA-1
自己写 WebSocket 握手
自己写 WebSocket frame 解析和编码
```

注意：这里的 SHA-1 是为了实现 WebSocket 协议握手，不是为了让你自己设计加密系统。

真实项目不要自己写密码学库。

## 这一阶段的目标

完成后，你会实现：

- `GET /ws` WebSocket endpoint
- HTTP Upgrade 请求识别
- `Sec-WebSocket-Accept` 计算
- `101 Switching Protocols` 响应
- 最小 Base64 编码
- 最小 SHA-1 哈希
- 读取客户端 WebSocket text frame
- 发送服务端 WebSocket text frame
- 实现 Echo Server
- 用浏览器页面测试

## WebSocket 和 SSE 的区别

SSE：

```text
服务端 -> 客户端
单向推送
基于 HTTP response body
文本事件流
```

WebSocket：

```text
客户端 <-> 服务端
双向通信
先 HTTP Upgrade
再切换成 WebSocket frame
二进制帧格式
```

SSE 适合：

```text
服务端通知
日志流
大模型 token 流
任务进度
```

WebSocket 适合：

```text
聊天室
在线游戏
协同编辑
实时双向控制
```

## 第 1 步：理解 WebSocket 握手请求

浏览器连接 WebSocket 时，会先发一个 HTTP 请求。

比如：

```http
GET /ws HTTP/1.1
Host: 127.0.0.1:8080
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==
Sec-WebSocket-Version: 13

```

关键 headers：

```http
Upgrade: websocket
```

意思是：

```text
我想把这个 HTTP 连接升级成 WebSocket
```

```http
Connection: Upgrade
```

意思是：

```text
这个连接要进行协议升级
```

```http
Sec-WebSocket-Key: ...
```

这是客户端随机生成的 key。

服务端要根据它计算 `Sec-WebSocket-Accept`。

## 第 2 步：理解 WebSocket 握手响应

服务端同意升级时，返回：

```http
HTTP/1.1 101 Switching Protocols
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Accept: ...

```

状态码：

```text
101 Switching Protocols
```

意思是：

```text
协议切换成功
```

这之后，这条 TCP 连接里传的就不再是普通 HTTP 请求/响应了，而是 WebSocket frame。

## 第 3 步：Sec-WebSocket-Accept 怎么算

算法固定：

```text
1. 取客户端的 Sec-WebSocket-Key
2. 拼上固定 GUID
3. 对拼接结果做 SHA-1
4. 对 SHA-1 结果做 Base64
```

固定 GUID 是：

```text
258EAFA5-E914-47DA-95CA-C5AB0DC85B11
```

伪代码：

```text
accept = base64(sha1(key + GUID))
```

例子：

```text
key    = dGhlIHNhbXBsZSBub25jZQ==
accept = s3pPLMBiTxaQ9kYGzzhZRbK+xOo=
```

这是 WebSocket 协议文档里的经典测试值。

## 第 4 步：建议新增文件结构

这一阶段建议新增一个 `ws` 模块：

```text
src/
  main.rs
  http/
    ...
  ws/
    mod.rs
    crypto.rs
    handshake.rs
    frame.rs
```

含义：

```text
crypto.rs     Base64 和 SHA-1
handshake.rs  WebSocket 握手
frame.rs      WebSocket frame 解析和编码
mod.rs        ws 模块入口
```

然后在 `src/main.rs` 里声明：

```rust
mod http;
mod ws;
```

如果你现在还在完成模块拆分，先确保 HTTP Server 能跑，再加 `ws` 模块。

## 第 5 步：写 `src/ws/mod.rs`

```rust
// WebSocket 相关模块。
pub mod crypto;
pub mod frame;
pub mod handshake;

// 重新导出常用函数，让外部使用路径短一点。
pub use frame::{read_ws_frame, write_ws_text, WsFrame};
pub use handshake::{is_websocket_upgrade, websocket_accept_key};
```

## 第 6 步：先写 Base64

创建：

```text
src/ws/crypto.rs
```

先写 Base64。

Base64 的作用是把任意字节变成可打印文本。

WebSocket 握手里，SHA-1 的结果是 20 个字节。HTTP header 不能直接放任意二进制，所以要 Base64。

```rust
// Base64 字符表。
const BASE64_TABLE: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

// 把字节编码成 Base64 字符串。
pub fn base64_encode(input: &[u8]) -> String {
    // 输出字符串。
    let mut output = String::new();

    // 每 3 个字节会编码成 4 个 Base64 字符。
    let mut i = 0;

    while i < input.len() {
        // 第 1 个字节一定存在。
        let b0 = input[i];

        // 第 2 个字节可能不存在，不存在就先当作 0。
        let b1 = if i + 1 < input.len() { input[i + 1] } else { 0 };

        // 第 3 个字节可能不存在，不存在就先当作 0。
        let b2 = if i + 2 < input.len() { input[i + 2] } else { 0 };

        // 把 3 个 8-bit 字节合成一个 24-bit 数字。
        let triple = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);

        // 每 6 bit 取一段，得到 4 个索引。
        let c0 = ((triple >> 18) & 0b0011_1111) as usize;
        let c1 = ((triple >> 12) & 0b0011_1111) as usize;
        let c2 = ((triple >> 6) & 0b0011_1111) as usize;
        let c3 = (triple & 0b0011_1111) as usize;

        // 前两个字符一定有。
        output.push(BASE64_TABLE[c0] as char);
        output.push(BASE64_TABLE[c1] as char);

        // 如果原始输入还有第 2 个字节，就输出第 3 个字符。
        // 否则用 = 补位。
        if i + 1 < input.len() {
            output.push(BASE64_TABLE[c2] as char);
        } else {
            output.push('=');
        }

        // 如果原始输入还有第 3 个字节，就输出第 4 个字符。
        // 否则用 = 补位。
        if i + 2 < input.len() {
            output.push(BASE64_TABLE[c3] as char);
        } else {
            output.push('=');
        }

        // 处理下一组 3 字节。
        i += 3;
    }

    output
}
```

Base64 测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_base64() {
        assert_eq!(base64_encode(b"hello"), "aGVsbG8=");
        assert_eq!(base64_encode(b"abc"), "YWJj");
    }
}
```

## 第 7 步：写最小 SHA-1

继续在 `src/ws/crypto.rs` 里追加：

```rust
// 计算 SHA-1。
//
// 输入：任意字节
// 输出：20 字节摘要
pub fn sha1(input: &[u8]) -> [u8; 20] {
    // SHA-1 初始常量。
    let mut h0: u32 = 0x67452301;
    let mut h1: u32 = 0xEFCDAB89;
    let mut h2: u32 = 0x98BADCFE;
    let mut h3: u32 = 0x10325476;
    let mut h4: u32 = 0xC3D2E1F0;

    // 复制输入，准备做 padding。
    let mut message = input.to_vec();

    // 原始消息长度，单位是 bit。
    let bit_len = (message.len() as u64) * 8;

    // 先追加一个 1 bit。
    // 0x80 的二进制是 10000000。
    message.push(0x80);

    // 再追加 0，直到长度模 64 等于 56。
    //
    // 为什么是 56？
    // 因为最后还要放 8 字节的原始长度。
    // 56 + 8 = 64，刚好一个块。
    while message.len() % 64 != 56 {
        message.push(0);
    }

    // 追加原始长度，使用大端序。
    message.extend_from_slice(&bit_len.to_be_bytes());

    // 每 64 字节处理一个块。
    for chunk in message.chunks(64) {
        // w 是 80 个 32-bit word。
        let mut w = [0u32; 80];

        // 前 16 个 word 直接来自 chunk。
        for i in 0..16 {
            let j = i * 4;
            w[i] = u32::from_be_bytes([
                chunk[j],
                chunk[j + 1],
                chunk[j + 2],
                chunk[j + 3],
            ]);
        }

        // 后 64 个 word 由前面的 word 推导出来。
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }

        // 工作变量。
        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        let mut e = h4;

        // SHA-1 主循环，共 80 轮。
        for i in 0..80 {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };

            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[i]);

            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        // 把这一块的结果加回总状态。
        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }

    // 输出 20 字节。
    let mut out = [0u8; 20];
    out[0..4].copy_from_slice(&h0.to_be_bytes());
    out[4..8].copy_from_slice(&h1.to_be_bytes());
    out[8..12].copy_from_slice(&h2.to_be_bytes());
    out[12..16].copy_from_slice(&h3.to_be_bytes());
    out[16..20].copy_from_slice(&h4.to_be_bytes());
    out
}
```

再加一个测试辅助函数：

```rust
// 把字节转成十六进制字符串，只用于测试。
fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::new();

    for byte in bytes {
        s.push_str(&format!("{:02x}", byte));
    }

    s
}
```

SHA-1 测试：

```rust
#[test]
fn hashes_sha1() {
    let digest = sha1(b"abc");
    assert_eq!(
        to_hex(&digest),
        "a9993e364706816aba3e25717850c26c9cd0d89d"
    );
}
```

## 第 8 步：实现 Sec-WebSocket-Accept

创建：

```text
src/ws/handshake.rs
```

写入：

```rust
use crate::http::Request;

use super::crypto::{base64_encode, sha1};

// WebSocket 协议规定的固定 GUID。
const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

// 根据客户端 Sec-WebSocket-Key 计算 Sec-WebSocket-Accept。
pub fn websocket_accept_key(key: &str) -> String {
    // 拼接 key 和固定 GUID。
    let combined = format!("{}{}", key.trim(), WS_GUID);

    // 对拼接结果做 SHA-1。
    let digest = sha1(combined.as_bytes());

    // 再做 Base64。
    base64_encode(&digest)
}

// 判断请求是不是 WebSocket Upgrade 请求。
pub fn is_websocket_upgrade(request: &Request) -> bool {
    // 必须是 GET。
    if request.method != "GET" {
        return false;
    }

    // 必须有 Upgrade: websocket。
    let Some(upgrade) = request.header("Upgrade") else {
        return false;
    };

    if !upgrade.eq_ignore_ascii_case("websocket") {
        return false;
    }

    // 必须有 Sec-WebSocket-Key。
    request.header("Sec-WebSocket-Key").is_some()
}
```

测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_accept_key() {
        let key = "dGhlIHNhbXBsZSBub25jZQ==";
        assert_eq!(
            websocket_accept_key(key),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
    }
}
```

这个测试很重要。

如果它通过，说明你的 Base64 + SHA-1 + WebSocket accept 算法是对的。

## 第 9 步：返回 101 Switching Protocols

在 `src/http/server.rs` 里，后面会需要一个函数：

```rust
use crate::ws::{is_websocket_upgrade, websocket_accept_key};
```

然后写：

```rust
// 处理 WebSocket 握手。
fn handle_websocket_handshake(
    stream: &mut TcpStream,
    request: &Request,
) -> std::io::Result<()> {
    // 取客户端发来的 Sec-WebSocket-Key。
    let key = request
        .header("Sec-WebSocket-Key")
        .expect("missing Sec-WebSocket-Key");

    // 计算 Sec-WebSocket-Accept。
    let accept = websocket_accept_key(key);

    // 构造 101 响应。
    let response = format!(
        concat!(
            "HTTP/1.1 101 Switching Protocols\r\n",
            "Upgrade: websocket\r\n",
            "Connection: Upgrade\r\n",
            "Sec-WebSocket-Accept: {}\r\n",
            "\r\n"
        ),
        accept
    );

    // 写回握手响应。
    stream.write_all(response.as_bytes())
}
```

注意：

```http
101 Switching Protocols
```

这不是普通 `200 OK`。

它表示协议切换成功。

## 第 10 步：理解 WebSocket Frame

握手之后，连接里传的是 WebSocket frame。

客户端发来的一个小文本 frame 大概结构是：

```text
byte 0:
  FIN  + opcode

byte 1:
  MASK + payload length

如果 payload length 较大：
  后面还会有扩展长度

如果 MASK = 1：
  后面有 4 字节 masking key

最后：
  payload data
```

你先记住几个值：

```text
opcode 0x1 = text
opcode 0x8 = close
opcode 0x9 = ping
opcode 0xA = pong
```

非常重要：

```text
浏览器 -> 服务端：payload 必须 mask
服务端 -> 浏览器：payload 不需要 mask
```

所以我们读客户端 frame 时要解 mask。

发送服务端 frame 时不要加 mask。

## 第 11 步：写 `src/ws/frame.rs`

先定义 frame 类型：

```rust
use std::io::{Read, Write};
use std::net::TcpStream;

// 我们当前阶段关心的 WebSocket frame。
#[derive(Debug)]
pub enum WsFrame {
    // 文本消息。
    Text(String),

    // 关闭连接。
    Close,

    // Ping。
    Ping(Vec<u8>),

    // Pong。
    Pong(Vec<u8>),

    // 暂不支持的其他 frame。
    Other,
}
```

## 第 12 步：读取客户端 frame

继续在 `frame.rs` 里写：

```rust
// 从 TCP 连接读取一个 WebSocket frame。
pub fn read_ws_frame(stream: &mut TcpStream) -> std::io::Result<WsFrame> {
    // WebSocket frame 至少有 2 字节头部。
    let mut header = [0u8; 2];
    stream.read_exact(&mut header)?;

    // 第一个字节。
    let byte0 = header[0];

    // 第二个字节。
    let byte1 = header[1];

    // FIN 表示这是不是消息的最后一帧。
    let fin = byte0 & 0b1000_0000 != 0;

    // opcode 是低 4 bit。
    let opcode = byte0 & 0b0000_1111;

    // MASK 位表示 payload 是否被 mask。
    let masked = byte1 & 0b1000_0000 != 0;

    // payload length 的基础值是低 7 bit。
    let mut payload_len = (byte1 & 0b0111_1111) as u64;

    // 这一阶段先不支持分片消息。
    if !fin {
        return Ok(WsFrame::Other);
    }

    // 如果长度是 126，后面 2 字节表示真实长度。
    if payload_len == 126 {
        let mut extended = [0u8; 2];
        stream.read_exact(&mut extended)?;
        payload_len = u16::from_be_bytes(extended) as u64;
    }

    // 如果长度是 127，后面 8 字节表示真实长度。
    if payload_len == 127 {
        let mut extended = [0u8; 8];
        stream.read_exact(&mut extended)?;
        payload_len = u64::from_be_bytes(extended);
    }

    // 浏览器发给服务端的 frame 必须 masked。
    if !masked {
        return Ok(WsFrame::Other);
    }

    // 读取 4 字节 masking key。
    let mut mask_key = [0u8; 4];
    stream.read_exact(&mut mask_key)?;

    // 读取 payload。
    let mut payload = vec![0u8; payload_len as usize];
    stream.read_exact(&mut payload)?;

    // 解 mask。
    for i in 0..payload.len() {
        payload[i] ^= mask_key[i % 4];
    }

    // 根据 opcode 解释 payload。
    match opcode {
        0x1 => {
            // text frame。
            let text = String::from_utf8_lossy(&payload).to_string();
            Ok(WsFrame::Text(text))
        }
        0x8 => Ok(WsFrame::Close),
        0x9 => Ok(WsFrame::Ping(payload)),
        0xA => Ok(WsFrame::Pong(payload)),
        _ => Ok(WsFrame::Other),
    }
}
```

这里用到了：

```rust
stream.read_exact(&mut header)?;
```

和之前的 `read` 不同：

```text
read       读到多少算多少
read_exact 必须读满整个 buffer，否则报错
```

读 WebSocket frame 头部时，我们明确知道要读 2 字节，所以用 `read_exact`。

## 第 13 步：发送服务端 text frame

继续在 `frame.rs` 里写：

```rust
// 向客户端发送一个 text frame。
pub fn write_ws_text(stream: &mut TcpStream, text: &str) -> std::io::Result<()> {
    // 文本内容转成字节。
    let payload = text.as_bytes();

    // frame 字节。
    let mut frame = Vec::new();

    // 第一个字节：
    // FIN = 1，opcode = 0x1 text。
    //
    // 1000_0001 = 0x81
    frame.push(0x81);

    // 服务端发给浏览器的 frame 不需要 mask。
    //
    // payload 长度有三种写法：
    // 0..=125       直接写长度
    // 126..=65535   先写 126，再写 2 字节长度
    // 更大          先写 127，再写 8 字节长度
    if payload.len() <= 125 {
        frame.push(payload.len() as u8);
    } else if payload.len() <= 65535 {
        frame.push(126);
        frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
    } else {
        frame.push(127);
        frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
    }

    // 追加 payload。
    frame.extend_from_slice(payload);

    // 写回 TCP 连接。
    stream.write_all(&frame)
}
```

服务端不 mask，这点很重要。

浏览器收到服务端 masked frame 反而会认为协议错误。

## 第 14 步：WebSocket Echo Loop

在 `src/http/server.rs` 里加：

```rust
use crate::ws::{read_ws_frame, write_ws_text, WsFrame};
```

然后写：

```rust
// WebSocket echo 循环。
fn websocket_echo_loop(mut stream: TcpStream) {
    loop {
        // 读取一个 WebSocket frame。
        let frame = match read_ws_frame(&mut stream) {
            Ok(frame) => frame,
            Err(error) => {
                println!("WebSocket read error: {}", error);
                break;
            }
        };

        match frame {
            WsFrame::Text(text) => {
                println!("WebSocket text: {}", text);

                // 收到什么，就回什么。
                if write_ws_text(&mut stream, &text).is_err() {
                    println!("WebSocket write failed");
                    break;
                }
            }
            WsFrame::Close => {
                println!("WebSocket closed");
                break;
            }
            WsFrame::Ping(payload) => {
                println!("WebSocket ping: {} bytes", payload.len());
            }
            WsFrame::Pong(payload) => {
                println!("WebSocket pong: {} bytes", payload.len());
            }
            WsFrame::Other => {
                println!("Unsupported WebSocket frame");
                break;
            }
        }
    }
}
```

这一阶段先不自动回复 Pong。

后面可以补。

## 第 15 步：在 `handle_connection` 里接入 `/ws`

在 `src/http/server.rs` 里，`handle_connection` 加 WebSocket 分支：

```rust
fn handle_connection(mut stream: TcpStream) {
    let Some(raw_request) = read_http_request(&mut stream) else {
        return;
    };

    println!(
        "Raw request:\n{}",
        String::from_utf8_lossy(&raw_request)
    );

    let request = parse_request(&raw_request).unwrap();

    // 如果是 WebSocket Upgrade 请求，就走 WebSocket。
    if request.path == "/ws" && is_websocket_upgrade(&request) {
        if handle_websocket_handshake(&mut stream, &request).is_err() {
            return;
        }

        websocket_echo_loop(stream);
        return;
    }

    // 如果是 SSE，继续走你上一阶段的 handle_sse。
    if request.method == "GET" && request.path == "/events" {
        handle_sse(stream);
        return;
    }

    let response = route(&request);
    let response_bytes = response.to_http_bytes();

    stream.write_all(&response_bytes).unwrap();
}
```

这里有一个重要顺序：

```text
先判断 /ws
再判断 /events
最后普通 HTTP
```

因为 `/ws` 握手成功后，这条连接会切换协议，不能再走普通 HTTP response。

## 第 16 步：测试 SHA-1、Base64、握手

先跑：

```bash
cargo test
```

你至少应该看到：

```text
encodes_base64 ... ok
hashes_sha1 ... ok
computes_accept_key ... ok
```

如果 `computes_accept_key` 不通过，先不要调 WebSocket frame。

先把握手算法修对。

因为浏览器 WebSocket 连接第一步就是检查 `Sec-WebSocket-Accept`。

## 第 17 步：用 HTML 页面测试

创建一个测试文件：

```text
examples/ws-client.html
```

内容：

```html
<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <title>WebSocket Test</title>
  </head>
  <body>
    <input id="message" value="hello websocket" />
    <button id="send">Send</button>
    <pre id="log"></pre>

    <script>
      const log = document.querySelector("#log");
      const input = document.querySelector("#message");
      const button = document.querySelector("#send");

      const ws = new WebSocket("ws://127.0.0.1:8080/ws");

      ws.onopen = () => {
        log.textContent += "connected\n";
      };

      ws.onmessage = (event) => {
        log.textContent += "server: " + event.data + "\n";
      };

      ws.onclose = () => {
        log.textContent += "closed\n";
      };

      ws.onerror = () => {
        log.textContent += "error\n";
      };

      button.onclick = () => {
        ws.send(input.value);
        log.textContent += "client: " + input.value + "\n";
      };
    </script>
  </body>
</html>
```

然后：

```bash
cargo run
```

用浏览器打开：

```text
examples/ws-client.html
```

点击 Send。

如果成功，你会看到：

```text
client: hello websocket
server: hello websocket
```

## 第 18 步：你应该理解的问题

完成后，你应该能回答：

1. WebSocket 为什么一开始是 HTTP 请求？
2. `101 Switching Protocols` 表示什么？
3. `Sec-WebSocket-Accept` 是怎么从 `Sec-WebSocket-Key` 算出来的？
4. Base64 在握手里解决了什么问题？
5. SHA-1 在握手里起什么作用？
6. WebSocket frame 的 `opcode` 是什么？
7. 为什么浏览器发给服务端的 frame 必须 mask？
8. 为什么服务端发给浏览器的 frame 不 mask？
9. `read_exact` 和 `read` 有什么区别？
10. 为什么握手之后不能再返回普通 HTTP response？

## 当前版本的局限

这个 WebSocket 仍然是教学版：

- 不支持 fragmented frame
- 不完整支持 ping/pong
- 没有限制最大 payload 长度
- 没有处理二进制消息
- 没有优雅发送 close frame
- 没有 TLS，所以只能用 `ws://`，不能用 `wss://`
- 每个连接一个线程，不适合大量连接

但它已经覆盖了 WebSocket 最核心的底层机制：

```text
HTTP Upgrade
Sec-WebSocket-Accept
WebSocket frame parse
mask/unmask
text echo
```

下一阶段可以继续做：

```text
完善 WebSocket：ping/pong、close frame、简单聊天室
```

或者进入更底层：

```text
教学版 TCP
```

