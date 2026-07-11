# 项目待办

## 下一步
- [ ] 将 `docs/08-mini-tcp-state-machine.md` 落成 `src/mini_tcp/` 代码和测试，并修正文档示例中服务端数据 ACK 的 `seq=500` 为 `seq=501`。
- [ ] 为 HTTP 请求跨多个 TCP read 的情况补测试，并修正 header 文本切片使用 `buffer[..bytes_read]` 的问题。
- [ ] 为 WebSocket frame parser/writer 增加纯字节级单元测试，避免测试依赖真实 `TcpStream`。

## 进行中
- [ ] 补齐 WebSocket 教学子集：Ping 返回 Pong、Close handshake、长度为 127 的 64 位 payload length。

## 待确认
- [ ] 是否先完成路线图中缺失的 HTTP Client，再继续 Mini TCP（README 顺序包含 HTTP Client，但现有阶段文档直接进入 SSE/WebSocket）。

## 阻塞

## 后续可做
- [ ] 将协议编解码与 socket I/O 解耦，便于测试并为“HTTP 跑在自制 TCP 上”做准备。
- [ ] 清理 `cargo clippy --all-targets -- -D warnings` 报告的未使用代码和风格问题。
- [ ] 给 HTTP 请求大小、WebSocket payload 长度和并发连接增加上限，避免无界内存/线程增长。
