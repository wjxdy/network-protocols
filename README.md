# Network Protocols From Zero

这是一个用 Rust 从零学习网络协议的项目。

目标不是一上来写出工业级网络库，而是通过亲手实现这些协议，真正理解它们在底层如何工作：

- TCP
- HTTP/1.1
- Server-Sent Events, SSE
- WebSocket

推荐学习顺序是：

```text
Rust 网络编程基础
  -> HTTP Server
  -> HTTP Client
  -> SSE
  -> WebSocket
  -> 教学版 TCP
  -> 尝试把 HTTP 跑在自制 TCP 上
```

## 先读什么

如果你是从零开始，建议按下面顺序读：

1. [docs/00-roadmap.md](docs/00-roadmap.md)
2. [docs/01-rust-basics-for-networking.md](docs/01-rust-basics-for-networking.md)
3. [docs/02-build-http-server-from-zero.md](docs/02-build-http-server-from-zero.md)
4. [docs/03-structure-http-server.md](docs/03-structure-http-server.md)
5. [docs/04-http-request-body-and-post.md](docs/04-http-request-body-and-post.md)

## 项目原则

这个项目会尽量少用第三方库。

前期允许使用 Rust 标准库里的 `std::net::TcpListener` 和 `std::net::TcpStream`，因为我们第一阶段要学习的是 HTTP、SSE、WebSocket 这些基于 TCP 字节流的应用层协议。

暂时不使用：

- `hyper`
- `tokio`
- `reqwest`
- `axum`
- `actix-web`
- `tungstenite`

也就是说：底层 TCP 连接先交给操作系统，协议格式、解析、状态机、请求响应都由我们自己写。

等 HTTP、SSE、WebSocket 跑通之后，再单独进入 TCP 阶段。
