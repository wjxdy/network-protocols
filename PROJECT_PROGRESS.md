# 项目进度

## 基本信息
- 项目名称：Network Protocols From Zero
- 当前阶段：开发（HTTP / SSE / WebSocket 已有实现，准备进入教学版 TCP）
- 最后更新：2026-07-11

## 当前状态
- 使用 Rust 标准库从零实现协议，当前二进制启动 HTTP Server，并包含 SSE 与 WebSocket echo 路径。
- `cargo test` 通过 7 个单元测试；`cargo clippy --all-targets -- -D warnings` 尚未通过。

## 已完成
- HTTP/1.1 请求解析、响应构造与基础路由。
- POST body 与 `Content-Length` 读取主线。
- SSE 事件流。
- WebSocket Upgrade、SHA-1/Base64、基础帧读取与文本 echo。
- 教学版 Mini TCP 状态机文档。

## 关键决策
- 2026-07-11: 先使用操作系统 TCP 学习 HTTP/SSE/WebSocket，再以内存状态机学习 TCP，最后尝试分层整合。
- 2026-07-11: 前期尽量只用标准库，不依赖成熟协议框架。

## 技术 / 结构备注
- 代码按 `http`、`ws` 模块拆分，教程按阶段保存在 `docs/`。
- 当前 WebSocket 实现是教学子集，不是 RFC 完整实现。

## 最近一次进展
- 2026-07-11: 完成项目体检；确认测试通过，并记录 HTTP 分片读取、WebSocket 大帧长度和协议完整性等后续问题。
