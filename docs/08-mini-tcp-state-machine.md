# 第 7 阶段：实现教学版 TCP 状态机

这一阶段正式进入 TCP。

但第一版不直接碰真实网卡、TUN、raw socket，也不直接接浏览器。

我们先做一个：

```text
内存里的 Mini TCP 状态机模拟器
```

目标是先把 TCP 最核心的机制写清楚：

```text
Segment
SYN / ACK
三次握手
连接状态
seq / ack
发送数据
确认数据
```

这一步不是绕路。

真正的 TCP 难点不是“把字节写到网卡”，而是：

```text
收到什么包
当前是什么状态
应该发什么包
seq/ack 怎么变
状态怎么变
```

这正是状态机模拟器要解决的。

## 这一阶段的目标

完成后，你会实现：

- 教学版 `Segment`
- TCP flags: `SYN`、`ACK`、`FIN`
- TCP 状态枚举 `TcpState`
- 客户端 endpoint
- 服务端 endpoint
- 三次握手
- `Established` 状态
- 客户端发送 `hello`
- 服务端返回 ACK
- 打印每一步状态变化

这一阶段暂时不做：

- 真实 TCP header
- IP header
- checksum
- 丢包重传
- 乱序重组
- 滑动窗口
- 拥塞控制
- TUN/raw socket

这些后面再做。

## 第 1 步：先纠正一个认知

应用层看到 TCP 时，感觉 TCP 是：

```text
一条可靠、有序、双向的字节流
```

但 TCP 底层并不是直接传“一条流”。

TCP 底层实际传的是一个个：

```text
TCP segment
```

每个 segment 大概包含：

```text
源端口
目标端口
seq
ack
flags
window
payload
```

TCP 做的事情是：

```text
把应用层 write 的字节拆成 segment
给每段字节编号 seq
对方收到后用 ack 确认
如果没确认就重传
如果乱序就重排
最后给应用层一个有序字节流
```

这一阶段先学最小核心：

```text
Segment + 状态机 + seq/ack
```

## 第 2 步：推荐文件结构

建议新建：

```text
src/
  mini_tcp/
    mod.rs
    segment.rs
    state.rs
    endpoint.rs
    simulation.rs
```

含义：

```text
segment.rs    TCP segment 的教学版结构
state.rs      TCP 状态枚举
endpoint.rs   Client/Server endpoint 状态机
simulation.rs 跑一遍握手和数据传输
mod.rs        mini_tcp 模块入口
```

然后在 `src/main.rs` 里临时加：

```rust
mod http;
mod ws;
mod mini_tcp;

fn main() {
    mini_tcp::simulation::run();
}
```

这样这一阶段先运行 TCP 模拟器。

等你要回到 HTTP/WebSocket server，再改回：

```rust
fn main() {
    http::server::serve("127.0.0.1:8080");
}
```

## 第 3 步：定义 Segment

创建：

```text
src/mini_tcp/segment.rs
```

写入：

```rust
// Segment 是教学版 TCP 包。
//
// 真实 TCP header 比这个复杂很多。
// 这里我们只保留理解三次握手和 seq/ack 需要的字段。
#[derive(Debug, Clone)]
pub struct Segment {
    // 源端口。
    pub src_port: u16,

    // 目标端口。
    pub dst_port: u16,

    // sequence number。
    //
    // 表示：这个 segment 里的数据，从发送方字节流的哪个编号开始。
    pub seq: u32,

    // acknowledgment number。
    //
    // 表示：我期望你下一次发来的字节编号。
    pub ack: u32,

    // SYN flag。
    //
    // 用于建立连接。
    pub syn: bool,

    // ACK flag。
    //
    // 表示 ack 字段有效。
    pub ack_flag: bool,

    // FIN flag。
    //
    // 用于关闭连接。这一阶段先定义，后面再实现关闭。
    pub fin: bool,

    // payload 是真正携带的数据。
    pub payload: Vec<u8>,
}

impl Segment {
    // 创建一个普通空 segment。
    pub fn new(src_port: u16, dst_port: u16) -> Segment {
        Segment {
            src_port,
            dst_port,
            seq: 0,
            ack: 0,
            syn: false,
            ack_flag: false,
            fin: false,
            payload: Vec::new(),
        }
    }

    // 计算这个 segment 会消耗多少 sequence number。
    pub fn seq_len(&self) -> u32 {
        // payload 每个字节消耗一个 seq。
        let mut len = self.payload.len() as u32;

        // SYN 会消耗一个 seq。
        if self.syn {
            len += 1;
        }

        // FIN 也会消耗一个 seq。
        if self.fin {
            len += 1;
        }

        len
    }

    // 生成一个方便打印的 flags 字符串。
    pub fn flags(&self) -> String {
        let mut flags = Vec::new();

        if self.syn {
            flags.push("SYN");
        }

        if self.ack_flag {
            flags.push("ACK");
        }

        if self.fin {
            flags.push("FIN");
        }

        if flags.is_empty() {
            "NONE".to_string()
        } else {
            flags.join("|")
        }
    }
}
```

### 你需要理解的点

这里最重要的是：

```rust
pub fn seq_len(&self) -> u32
```

TCP 里：

```text
payload 里的每个字节消耗一个 seq
SYN 消耗一个 seq
FIN 消耗一个 seq
纯 ACK 不消耗 seq
```

比如：

```text
SYN seq=100
对方 ack=101
```

因为 SYN 消耗一个序列号。

再比如：

```text
payload = "hello"
seq = 101
payload 长度 = 5
对方 ack = 106
```

因为对方已经收到了：

```text
101, 102, 103, 104, 105
```

下一个期待的是：

```text
106
```

## 第 4 步：定义 TCP 状态

创建：

```text
src/mini_tcp/state.rs
```

写入：

```rust
// 教学版 TCP 状态。
//
// 真实 TCP 状态更多。
// 第一版只实现建立连接需要的状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    // 初始关闭状态。
    Closed,

    // 服务端监听状态。
    Listen,

    // 客户端发送 SYN 后，等待 SYN+ACK。
    SynSent,

    // 服务端收到 SYN 并发送 SYN+ACK 后，等待最终 ACK。
    SynReceived,

    // 连接建立完成。
    Established,
}
```

`PartialEq` 和 `Eq` 是为了后面测试状态是否相等：

```rust
assert_eq!(client.state, TcpState::Established);
```

## 第 5 步：定义 Endpoint

创建：

```text
src/mini_tcp/endpoint.rs
```

先写结构：

```rust
use super::segment::Segment;
use super::state::TcpState;

// EndpointRole 用来区分客户端和服务端。
#[derive(Debug, Clone, Copy)]
pub enum EndpointRole {
    Client,
    Server,
}

// TcpEndpoint 表示一个 TCP 端点。
//
// 一个客户端是一个 endpoint。
// 一个服务端也是一个 endpoint。
pub struct TcpEndpoint {
    // 用来打印日志。
    pub name: String,

    // 端点角色。
    pub role: EndpointRole,

    // 当前 TCP 状态。
    pub state: TcpState,

    // 本地端口。
    pub local_port: u16,

    // 对端端口。
    pub remote_port: u16,

    // 我下一次要发送的 sequence number。
    pub send_next: u32,

    // 我期望对方下一次发来的 sequence number。
    pub recv_next: u32,

    // 收到的应用数据。
    pub received_data: Vec<u8>,
}
```

这里有两个核心字段：

```rust
send_next
recv_next
```

含义：

```text
send_next = 我下一个要发出去的字节编号
recv_next = 我期望对方下一个发来的字节编号
```

## 第 6 步：实现构造函数

继续在 `endpoint.rs` 里写：

```rust
impl TcpEndpoint {
    // 创建客户端。
    pub fn new_client(local_port: u16, remote_port: u16, initial_seq: u32) -> TcpEndpoint {
        TcpEndpoint {
            name: "client".to_string(),
            role: EndpointRole::Client,
            state: TcpState::Closed,
            local_port,
            remote_port,
            send_next: initial_seq,
            recv_next: 0,
            received_data: Vec::new(),
        }
    }

    // 创建服务端。
    pub fn new_server(local_port: u16, initial_seq: u32) -> TcpEndpoint {
        TcpEndpoint {
            name: "server".to_string(),
            role: EndpointRole::Server,
            state: TcpState::Listen,
            local_port,
            remote_port: 0,
            send_next: initial_seq,
            recv_next: 0,
            received_data: Vec::new(),
        }
    }
}
```

这里的 `initial_seq` 是初始序列号。

真实 TCP 的初始序列号通常不是固定的，会随机化。

教学版先手动传入：

```text
client initial seq = 100
server initial seq = 500
```

这样日志容易看。

## 第 7 步：客户端发 SYN

继续在 `endpoint.rs` 里写：

```rust
impl TcpEndpoint {
    // 客户端主动发起连接。
    pub fn connect(&mut self) -> Segment {
        // 只有客户端 Closed 状态可以主动 connect。
        assert_eq!(self.state, TcpState::Closed);

        // 构造 SYN segment。
        let mut syn = Segment::new(self.local_port, self.remote_port);
        syn.seq = self.send_next;
        syn.syn = true;

        println!(
            "{} -> SYN seq={}",
            self.name,
            syn.seq
        );

        // SYN 消耗一个 seq。
        self.send_next += syn.seq_len();

        // 状态变成 SynSent。
        self.state = TcpState::SynSent;

        syn
    }
}
```

调用后：

```text
client state: Closed -> SynSent
client send_next: 100 -> 101
```

## 第 8 步：服务端收到 SYN，返回 SYN+ACK

继续写：

```rust
impl TcpEndpoint {
    // 服务端处理收到的 segment。
    pub fn on_segment(&mut self, segment: Segment) -> Option<Segment> {
        println!(
            "{} <- flags={} seq={} ack={} len={}",
            self.name,
            segment.flags(),
            segment.seq,
            segment.ack,
            segment.payload.len()
        );

        match self.state {
            TcpState::Listen => self.on_listen(segment),
            TcpState::SynSent => self.on_syn_sent(segment),
            TcpState::SynReceived => self.on_syn_received(segment),
            TcpState::Established => self.on_established(segment),
            TcpState::Closed => None,
        }
    }

    fn on_listen(&mut self, segment: Segment) -> Option<Segment> {
        // Listen 状态只处理 SYN。
        if !segment.syn {
            return None;
        }

        // 记录客户端端口。
        self.remote_port = segment.src_port;

        // 服务端期望客户端下一个 seq。
        self.recv_next = segment.seq + segment.seq_len();

        // 构造 SYN+ACK。
        let mut syn_ack = Segment::new(self.local_port, self.remote_port);
        syn_ack.seq = self.send_next;
        syn_ack.ack = self.recv_next;
        syn_ack.syn = true;
        syn_ack.ack_flag = true;

        println!(
            "{} -> SYN+ACK seq={} ack={}",
            self.name,
            syn_ack.seq,
            syn_ack.ack
        );

        // SYN 消耗一个 seq。
        self.send_next += syn_ack.seq_len();

        // 状态变成 SynReceived。
        self.state = TcpState::SynReceived;

        Some(syn_ack)
    }
}
```

这里服务端做了几件事：

```text
收到 SYN seq=100
知道客户端下一个 seq 应该是 101
所以回复 ack=101
服务端自己的 SYN seq=500
发完后 send_next 变成 501
状态 Listen -> SynReceived
```

## 第 9 步：客户端收到 SYN+ACK，返回 ACK

继续写：

```rust
impl TcpEndpoint {
    fn on_syn_sent(&mut self, segment: Segment) -> Option<Segment> {
        // 客户端在 SynSent 状态，期待收到 SYN+ACK。
        if !(segment.syn && segment.ack_flag) {
            return None;
        }

        // 检查服务端 ack 是否确认了客户端 SYN。
        if segment.ack != self.send_next {
            println!(
                "{} ignored SYN+ACK: expected ack {}, got {}",
                self.name,
                self.send_next,
                segment.ack
            );
            return None;
        }

        // 客户端期望服务端下一个 seq。
        self.recv_next = segment.seq + segment.seq_len();

        // 构造最终 ACK。
        let mut ack = Segment::new(self.local_port, self.remote_port);
        ack.seq = self.send_next;
        ack.ack = self.recv_next;
        ack.ack_flag = true;

        println!(
            "{} -> ACK seq={} ack={}",
            self.name,
            ack.seq,
            ack.ack
        );

        // 纯 ACK 不消耗 seq，所以 send_next 不变。

        // 客户端进入 Established。
        self.state = TcpState::Established;

        Some(ack)
    }
}
```

这里客户端做了：

```text
收到服务端 SYN seq=500
SYN 消耗一个 seq
所以 ack=501
状态 SynSent -> Established
```

## 第 10 步：服务端收到最终 ACK

继续写：

```rust
impl TcpEndpoint {
    fn on_syn_received(&mut self, segment: Segment) -> Option<Segment> {
        // 服务端期待最终 ACK。
        if !segment.ack_flag {
            return None;
        }

        // 检查客户端 ack 是否确认了服务端 SYN。
        if segment.ack != self.send_next {
            println!(
                "{} ignored ACK: expected ack {}, got {}",
                self.name,
                self.send_next,
                segment.ack
            );
            return None;
        }

        // 服务端进入 Established。
        self.state = TcpState::Established;

        println!("{} state -> Established", self.name);

        None
    }
}
```

到这里三次握手完成：

```text
Client: Established
Server: Established
```

## 第 11 步：客户端发送 payload

继续写：

```rust
impl TcpEndpoint {
    // 发送应用数据。
    pub fn send_data(&mut self, data: &[u8]) -> Segment {
        assert_eq!(self.state, TcpState::Established);

        let mut segment = Segment::new(self.local_port, self.remote_port);
        segment.seq = self.send_next;
        segment.ack = self.recv_next;
        segment.ack_flag = true;
        segment.payload = data.to_vec();

        println!(
            "{} -> DATA seq={} ack={} payload={:?}",
            self.name,
            segment.seq,
            segment.ack,
            String::from_utf8_lossy(&segment.payload)
        );

        // payload 每个字节消耗一个 seq。
        self.send_next += segment.seq_len();

        segment
    }
}
```

如果客户端当前：

```text
send_next = 101
```

发送：

```text
hello
```

长度 5。

发送后：

```text
client send_next = 106
```

## 第 12 步：服务端收到数据，返回 ACK

继续写：

```rust
impl TcpEndpoint {
    fn on_established(&mut self, segment: Segment) -> Option<Segment> {
        // 如果 segment 有 payload，就接收数据并返回 ACK。
        if !segment.payload.is_empty() {
            // 检查 seq 是否是我期待的。
            if segment.seq != self.recv_next {
                println!(
                    "{} ignored out-of-order data: expected seq {}, got {}",
                    self.name,
                    self.recv_next,
                    segment.seq
                );
                return None;
            }

            // 保存数据。
            self.received_data.extend_from_slice(&segment.payload);

            // 推进 recv_next。
            self.recv_next += segment.seq_len();

            // 构造 ACK。
            let mut ack = Segment::new(self.local_port, self.remote_port);
            ack.seq = self.send_next;
            ack.ack = self.recv_next;
            ack.ack_flag = true;

            println!(
                "{} -> ACK seq={} ack={}",
                self.name,
                ack.seq,
                ack.ack
            );

            return Some(ack);
        }

        // 如果只是 ACK，第一版先只打印。
        if segment.ack_flag {
            println!(
                "{} received ACK ack={}",
                self.name,
                segment.ack
            );
        }

        None
    }
}
```

这里服务端收到：

```text
DATA seq=101 payload="hello"
```

因为 `hello` 长度 5，所以回复：

```text
ACK ack=106
```

## 第 13 步：模块入口

创建：

```text
src/mini_tcp/mod.rs
```

写入：

```rust
pub mod endpoint;
pub mod segment;
pub mod simulation;
pub mod state;
```

## 第 14 步：写 simulation

创建：

```text
src/mini_tcp/simulation.rs
```

写入：

```rust
use super::endpoint::TcpEndpoint;
use super::state::TcpState;

pub fn run() {
    // 客户端本地端口 40000，连接服务端 8080，初始 seq=100。
    let mut client = TcpEndpoint::new_client(40000, 8080, 100);

    // 服务端监听 8080，初始 seq=500。
    let mut server = TcpEndpoint::new_server(8080, 500);

    println!("--- three-way handshake ---");

    // 第一次握手：client -> server: SYN。
    let syn = client.connect();

    // 第二次握手：server -> client: SYN+ACK。
    let syn_ack = server.on_segment(syn).expect("server should reply SYN+ACK");

    // 第三次握手：client -> server: ACK。
    let ack = client.on_segment(syn_ack).expect("client should reply ACK");

    // 服务端收到最终 ACK。
    server.on_segment(ack);

    assert_eq!(client.state, TcpState::Established);
    assert_eq!(server.state, TcpState::Established);

    println!("--- data transfer ---");

    // 客户端发送 hello。
    let data = client.send_data(b"hello");

    // 服务端收到 hello，返回 ACK。
    let ack = server.on_segment(data).expect("server should ACK data");

    // 客户端收到 ACK。
    client.on_segment(ack);

    println!(
        "server received data: {:?}",
        String::from_utf8_lossy(&server.received_data)
    );

    assert_eq!(server.received_data, b"hello");
}
```

## 第 15 步：运行模拟器

临时把 `src/main.rs` 改成：

```rust
mod http;
mod mini_tcp;
mod ws;

fn main() {
    mini_tcp::simulation::run();
}
```

运行：

```bash
cargo run
```

你应该看到类似：

```text
--- three-way handshake ---
client -> SYN seq=100
server <- flags=SYN seq=100 ack=0 len=0
server -> SYN+ACK seq=500 ack=101
client <- flags=SYN|ACK seq=500 ack=101 len=0
client -> ACK seq=101 ack=501
server <- flags=ACK seq=101 ack=501 len=0
server state -> Established
--- data transfer ---
client -> DATA seq=101 ack=501 payload="hello"
server <- flags=ACK seq=101 ack=501 len=5
server -> ACK seq=500 ack=106
client <- flags=ACK seq=500 ack=106 len=0
client received ACK ack=106
server received data: "hello"
```

## 第 16 步：你应该理解的问题

完成后，你应该能回答：

1. TCP 为什么需要状态机？
2. `Closed`、`Listen`、`SynSent`、`SynReceived`、`Established` 分别代表什么？
3. `seq` 是什么？
4. `ack` 是什么？
5. 为什么 SYN 会消耗一个序列号？
6. 为什么纯 ACK 不消耗序列号？
7. 客户端发送 `hello`，为什么服务端 ACK 是 `seq + 5`？
8. TCP 底层传的是 segment，为什么应用层看到的是字节流？

## 当前版本的局限

这个 Mini TCP 只实现了最小主线：

```text
三次握手
Established
顺序数据
ACK
```

它还没有：

```text
真实 TCP header
checksum
丢包
重传
乱序缓存
滑动窗口
连接关闭
TIME_WAIT
拥塞控制
```

下一阶段建议做：

```text
Mini TCP 可靠传输：丢包、超时重传、乱序和简单窗口
```

那一阶段你会开始真正理解：

```text
TCP 为什么叫可靠传输
```

