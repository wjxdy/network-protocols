# 从零实现网络协议：开发路线图

这份文档是整个项目的地图。

你现在的目标是：用 Rust 从零开始，亲手实现 HTTP、SSE、WebSocket，最后再实现一个教学版 TCP。

这条路线不要求你一开始就懂 Rust，也不要求你一开始就懂网络协议。我们会按“能跑起来 -> 理解格式 -> 拆成模块 -> 再往底层走”的方式推进。

## 为什么不要一开始就写 TCP

很多人一听“从底层实现网络协议”，第一反应是：那就先从 TCP 开始。

这个想法很酷，但对初学者不太友好。

TCP 不只是一个包格式。它还包含：

- 三次握手
- 四次挥手
- 序列号 `seq`
- 确认号 `ack`
- 丢包重传
- 乱序处理
- 滑动窗口
- 拥塞控制
- 连接状态机
- TIME_WAIT

如果你 Rust 语法还不熟，一上来同时学 Rust、二进制协议、状态机、网络调试，会很容易卡住。

所以我们先从 HTTP Server 开始。

HTTP Server 的好处是：

- 能很快跑起来
- 可以直接用浏览器测试
- 协议格式是文本，容易看懂
- 能自然引出 TCP 字节流
- 后面能继续接 SSE 和 WebSocket

## 协议之间的关系

先记住这张关系图：

```text
SSE
  |
  v
HTTP/1.1
  |
  v
TCP

WebSocket
  |
  v
HTTP Upgrade
  |
  v
HTTP/1.1
  |
  v
TCP
```

SSE 是建立在 HTTP 上的长连接事件流。

WebSocket 一开始也是 HTTP 请求，服务端同意升级之后，连接才切换成 WebSocket 帧协议。

HTTP 是建立在 TCP 字节流上的应用层协议。

TCP 提供可靠、有序的字节流。

## 总体阶段

### 第 0 阶段：准备 Rust 和项目环境

目标：

- 安装 Rust
- 学会使用 `cargo`
- 知道 Rust 项目基本结构
- 能运行一个 `main.rs`

你需要掌握：

- `cargo new`
- `cargo run`
- `cargo check`
- `cargo test`
- `src/main.rs`
- `Cargo.toml`

这一阶段不追求写复杂代码，只要你能创建并运行一个 Rust 程序。

### 第 1 阶段：实现 HTTP Server

目标：

- 用 `TcpListener` 监听端口
- 接受浏览器或 `curl` 发来的请求
- 读取原始 HTTP 请求文本
- 手动构造 HTTP 响应
- 返回 `Hello, world!`
- 解析 method、path、version、headers
- 根据不同 path 返回不同内容

这是第一个真正的里程碑。

完成后，你会理解：

- 浏览器访问网站时，发出去的请求到底长什么样
- HTTP 响应为什么要有 status line、headers、body
- `Content-Length` 为什么重要
- TCP 连接里传输的其实是一串字节

### 第 2 阶段：实现 HTTP Client

目标：

- 用 `TcpStream::connect` 连接一个 HTTP Server
- 手写 `GET / HTTP/1.1` 请求
- 发送请求
- 读取响应
- 解析 status line、headers、body

完成后，你会明白浏览器、`curl`、HTTP 客户端本质上在做什么。

### 第 3 阶段：实现 SSE

目标：

- 实现 `Content-Type: text/event-stream`
- 保持 HTTP 连接不断开
- 服务端持续推送事件
- 客户端持续读取事件
- 支持 `data:`、`event:`、`id:`、空行分隔

SSE 的核心是：它不是一个全新的传输层协议，而是 HTTP response body 的一种流式格式。

### 第 4 阶段：实现 WebSocket

目标：

- 识别 HTTP Upgrade 请求
- 实现 WebSocket 握手
- 实现基本 frame 编码和解码
- 支持 text frame
- 支持 close frame
- 支持 ping/pong
- 做一个 echo server

WebSocket 比 SSE 更复杂，因为握手之后，数据不再是普通 HTTP body，而是 WebSocket 自己的二进制 frame。

### 第 5 阶段：实现教学版 TCP

目标：

- 先在内存里模拟网络，不直接操作真实网卡
- 定义 TCP segment
- 实现 client/server 状态机
- 实现三次握手
- 实现简单数据发送和 ACK
- 实现四次挥手
- 模拟丢包和重传

这一阶段的重点是理解 TCP 的可靠传输机制。

不要一开始就追求接入真实网络。先把状态机跑明白。

### 第 6 阶段：把 HTTP 跑在自制 TCP 上

目标：

- 把前面写的 HTTP parser 和 response builder 抽象成“读写字节流”
- 让 HTTP 不依赖 `TcpStream`
- 尝试用自制 TCP 的字节流承载 HTTP

这一步是最终整合。

如果能做到这里，你会真正理解：

- HTTP 为什么不关心底层包怎么重传
- TCP 为什么要提供可靠、有序的字节流
- 协议分层为什么重要

## 推荐目录结构

第一阶段先不要搞太复杂，可以这样开始：

```text
network-protocols/
  Cargo.toml
  src/
    main.rs
  docs/
    00-roadmap.md
    01-rust-basics-for-networking.md
    02-build-http-server-from-zero.md
```

等代码多起来之后，再拆成：

```text
src/
  main.rs
  http/
    mod.rs
    request.rs
    response.rs
    server.rs
```

再后面，如果做成多 crate 项目，可以升级为：

```text
crates/
  proto-core/
  mini-http/
  mini-sse/
  mini-websocket/
  mini-tcp/
examples/
```

但刚开始不要这样。初学阶段优先让代码跑起来。

## 学习原则

### 1. 先跑起来，再变漂亮

第一次写 HTTP Server 的代码可以粗糙一点。

比如刚开始可以直接在 `main.rs` 里写所有代码。

等你知道每一行在干什么之后，再拆成函数、结构体和模块。

### 2. 每一步都用工具验证

常用验证方式：

```bash
curl -v http://127.0.0.1:8080/
```

或者直接打开浏览器：

```text
http://127.0.0.1:8080/
```

不要只看代码，要看真实请求和响应。

### 3. 少用库，但不要拒绝标准库

我们暂时不用成熟协议库，但会使用 Rust 标准库。

标准库里的 TCP 是操作系统提供的能力。第一阶段用它是合理的，因为我们要先学习 HTTP。

### 4. 错误比成功更有价值

如果浏览器打不开、`curl` 卡住、响应乱码，不要急着重写。

先问：

- 服务端有没有监听端口？
- 请求有没有读到？
- 响应有没有写回？
- 响应格式有没有空行？
- `Content-Length` 是否正确？

网络协议学习的核心能力之一，就是学会观察字节和状态。

