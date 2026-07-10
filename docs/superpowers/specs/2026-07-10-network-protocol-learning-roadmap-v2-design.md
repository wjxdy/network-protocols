# 网络协议学习路线 V2 设计稿

## 1. 设计结论

这个项目不推翻已经完成的 HTTP、SSE 和 WebSocket 学习成果。

新版路线采用“两遍螺旋式学习”：

```text
第一遍：从应用层入门
HTTP -> SSE -> WebSocket

第二遍：从传输层往上重建
TCP 状态机
  -> TCP 真实报文
  -> 最小 IPv4
  -> 可靠传输
  -> 真实网络接入
  -> 再次承载 HTTP / SSE / WebSocket
  -> TLS / HTTPS / WSS
```

第一遍解决的问题是：

```text
应用程序怎样把字节组织成协议？
```

第二遍解决的问题是：

```text
TCP 怎样把不可靠的网络包变成可靠、有序的字节流？
```

最终整合解决的问题是：

```text
上层协议为什么可以不关心丢包、乱序和重传？
```

## 2. 学习目标

最终目标不是做一个可以替代操作系统 TCP 的生产级网络栈，而是亲手实现一个可以验证、可以观察、可以在受控环境中与真实程序互通的教学版协议栈。

项目完成后，学习者应当能够：

1. 解释 HTTP、SSE、WebSocket、TLS、TCP 和 IPv4 之间的依赖关系。
2. 手动解析和生成这些协议的核心报文。
3. 解释 TCP 三次握手、序列号、确认号、重传、乱序重组、流量控制和连接关闭。
4. 在模拟网络中主动制造丢包、延迟、重复和乱序，并观察 TCP 如何恢复。
5. 通过虚拟网络接口收发真实 IPv4 和 TCP 报文。
6. 把已有 HTTP 代码从 `std::net::TcpStream` 中解耦，并运行在自制 TCP 字节流上。
7. 清楚区分“协议实现练习”和“可以安全用于生产的实现”。

## 3. 项目边界

### 3.1 必须实现

- HTTP/1.1 最小 Server 和 Client。
- HTTP 请求与响应的增量读取和解析。
- SSE 事件流。
- WebSocket Upgrade、基础 Frame 和 Echo。
- TCP 连接状态机。
- TCP Header 编码与解码。
- TCP checksum。
- 最小 IPv4 Header 编码与解码。
- 超时重传和累计 ACK。
- 乱序缓存和有序交付。
- 基础接收窗口。
- TCP 正常关闭。
- 通过虚拟网络接口进行真实报文实验。
- HTTP 运行在自制 TCP 上。

### 3.2 后期选修

- TCP 慢启动和拥塞避免。
- 快速重传和快速恢复。
- TCP Options，例如 MSS 和 Window Scale。
- IPv4 分片。
- IPv6。
- 多连接和高并发。
- 完整 TLS 1.3 互操作。

### 3.3 明确不追求

- 替代操作系统网络栈。
- 覆盖 TCP 和 HTTP 的全部 RFC。
- 在公网中运行没有完整拥塞控制的自制 TCP。
- 把自己实现的密码学代码宣传为安全实现。
- 为了“完全不用库”而同时手写操作系统接口、密码学大整数和所有工程工具。

## 4. 协议依赖关系

不加密时：

```text
HTTP/1.1 -----------+
SSE -> HTTP/1.1 ----+-> TCP -> IPv4 -> 虚拟网卡或真实网络
WebSocket ----------+
  先用 HTTP Upgrade 握手
  再在同一条 TCP 连接上传输 WebSocket Frame
```

加密时：

```text
HTTPS = HTTP + TLS + TCP
WSS   = WebSocket + TLS + TCP
HTTPS 上的 SSE = SSE + HTTP + TLS + TCP
```

TLS 不是简单地“使用非对称加密传输所有数据”。

TLS 通常使用签名和密钥交换完成身份验证及共享秘密协商，再使用对称 AEAD 加密实际应用数据。

## 5. 当前学习基线

当前已经完成：

- Rust 项目、模块、结构体、枚举、`Result`、`Read`、`Write` 等基础。
- 使用 `TcpListener` 和 `TcpStream` 实现 HTTP Server。
- HTTP request line、headers、body 和 `Content-Length` 解析。
- POST 请求和增量读取。
- HTTP 模块拆分及基础单元测试。
- SSE 持续推送和每连接一个线程的并发模型。
- WebSocket Upgrade。
- SHA-1、Base64 和 `Sec-WebSocket-Accept`。
- WebSocket 基础 Frame、mask 还原和 Echo。

当前尚未完成：

- 自己的 HTTP Client 和 HTTP Response Parser。
- HTTP 分块传输解析。
- TCP 状态机源码。
- TCP 真实报文字节。
- IPv4、checksum、重传、乱序和窗口。
- TLS。

因此，新路线从一个很短的 HTTP Client 补全阶段开始，然后正式进入 TCP。

## 6. 新版阶段路线

### 阶段 7：补全最小 HTTP Client

目的：补上当前路线中只有 Server、没有 Client 的缺口，并从客户端角度再次理解响应边界。

学习内容：

- `TcpStream::connect`。
- 手动构造 GET 请求。
- HTTP status line 和 response headers。
- 根据 `Content-Length` 读取 response body。
- 服务端关闭连接时如何判断 body 结束。
- 把 `Transfer-Encoding: chunked` 作为本阶段的扩展任务；它不阻塞进入 TCP，但要在最终 HTTP 集成前完成。
- 暂不支持 HTTPS、重定向和连接池。

验收标准：

- 自己写的 Client 能请求自己写的 Server。
- 故意把响应拆成多次写入，Client 仍能读完整。
- Response Parser 有正常、截断和错误输入测试。
- 扩展任务可以解析由多个 chunk 组成的 response body 和结束块 `0\r\n\r\n`。

建议文档：

```text
docs/08-http-client-and-response-parser.md
```

### 阶段 8：TCP 0.1，状态机与序列空间

目的：只研究 TCP 的状态和数字如何变化，不处理真实网卡和报文字节。

学习内容：

- `Segment` 教学结构。
- `Closed`、`Listen`、`SynSent`、`SynReceived`、`Established`。
- 三次握手。
- `SND.UNA`：最早还没有被确认的序列号。
- `SND.NXT`：下一个要发送的序列号。
- `RCV.NXT`：下一个期望收到的序列号。
- SYN 为什么消耗一个序列号，并提前知道 FIN 也遵循相同规则。
- payload 为什么按字节消耗序列号。
- ACK 到达后如何推进 `SND.UNA`。

这一阶段明确不做：

- TCP Header。
- checksum。
- 重传。
- 乱序缓存。
- 滑动窗口。
- TUN。

验收标准：

- 客户端和服务端都进入 `Established`。
- 双方都可以发送并确认数据。
- 错误 ACK 不会错误推进发送状态。
- 每次状态变化和三个序列号变量都有单元测试。
- 学习者可以不用看代码，手算一次握手和一次 `hello` 传输。

建议把当前文档改名为：

```text
docs/09-tcp-state-machine-and-sequence-space.md
```

### 阶段 9：TCP 0.2，真实 Header 和字节编解码

目的：让内存中的 `Segment` 第一次变成真实 TCP 报文字节。

学习内容：

- 大端字节序。
- 源端口和目标端口。
- seq 和 ack。
- data offset。
- TCP flags 位图。
- window、checksum、urgent pointer。
- payload。
- `encode` 和 `decode`。
- 长度检查和非法 Header 错误。

验收标准：

- `Segment -> Vec<u8> -> Segment` 往返结果一致。
- 可以解析一组已知 TCP Header 字节。
- 对过短 Header、错误 data offset 和截断 payload 返回明确错误。
- 状态机仍只接收结构化 `Segment`，编解码模块保持独立。

### 阶段 10：最小 IPv4 和 TCP checksum

目的：补上 TCP 在真实网络中不能脱离 IP 独立存在的知识。

学习内容：

- IPv4 version、IHL、total length、protocol、source、destination。
- IPv4 Header checksum。
- TCP pseudo header。
- one's complement sum。
- TCP checksum 计算和验证。
- 暂不处理 IPv4 分片和 Options。

验收标准：

- 可以编码和解析最小 IPv4 Header。
- 修改 payload 任意一个 bit 后，TCP checksum 校验失败。
- 使用固定测试向量验证 checksum，而不只做“自己算、自己验证”的循环测试。

### 阶段 11：可控模拟网络与超时重传

目的：第一次实现 TCP 的“可靠”二字。

模拟网络负责：

- 正常投递。
- 丢弃指定报文。
- 延迟投递。
- 重复投递。
- 改变投递顺序。

TCP 端点新增：

- 发送缓冲区。
- 未确认报文记录。
- 模拟时钟。
- 重传定时器。
- 超时事件。
- ACK 到达后取消或重设定时器。

设计要求：

- 测试使用模拟时钟，不在单元测试里调用真实 `sleep`。
- 网络故障必须可以确定地复现。
- 状态机逐步改造成 `Event -> Vec<Action>`。

典型事件：

```text
ActiveOpen
PassiveOpen
SegmentArrived
SendData
Timeout
Close
```

其中 `Close` 先作为后续会出现的事件名称保留，直到阶段 13 才实现对应状态变化。

典型动作：

```text
SendSegment
StartTimer
CancelTimer
DeliverData
ConnectionEstablished
ConnectionClosed
```

验收标准：

- 第一个数据报文被丢弃后，可以超时重传并最终交付。
- ACK 被丢弃后，重复数据不会向应用层交付两次。
- 所有测试不依赖真实网络速度。

### 阶段 12：乱序重组与基础流量控制

目的：把一组可能乱序到达的 segment 恢复成应用层看到的有序字节流。

学习内容：

- 接收缓冲区。
- 重复 segment 去重。
- 乱序 segment 缓存。
- 累计 ACK。
- segment 拆分。
- advertised window。
- 基础滑动窗口。
- 序列号回绕的概念和比较方法。

验收标准：

- `world` 比 `hello ` 先到达，应用层最终仍只读到 `hello world`。
- 重复报文不会产生重复字节。
- 接收窗口为零时，发送方不会继续发送普通数据。
- 大于单个 segment 的数据能够拆分、确认和重组。

### 阶段 13：连接关闭与异常连接

目的：完整理解 TCP 连接不是一次握手后直接删除，而是双方独立关闭发送方向。

学习内容：

- FIN。
- 半关闭。
- `FinWait1`、`FinWait2`、`CloseWait`、`LastAck`、`TimeWait`。
- FIN 重传。
- RST 的最小处理。
- 为什么需要 TIME_WAIT。

验收标准：

- 主动关闭和被动关闭路径都有状态测试。
- FIN 或最终 ACK 丢失时可以恢复。
- 一方关闭发送方向后，仍能接收对方剩余数据。

### 阶段 14：接入真实网络报文

目的：把纯内存协议栈接到虚拟三层网络接口，开始接收真实 IPv4 packet。

推荐环境：

- 状态机、编解码和模拟网络阶段继续在当前系统开发。
- TUN 实验优先放在隔离的 Linux 环境中。
- macOS `utun` 适配放在以后，不让平台接口干扰 TCP 主线。

模块边界：

```text
TunDevice
  -> 读取 IPv4 bytes
Ipv4Codec
  -> 得到 TCP bytes
TcpCodec
  -> 得到 Segment
TcpStack
  -> 处理事件并产生 Action
TunDevice
  -> 写回 IPv4 bytes
```

验收标准：

- 抓包工具能看到自制协议栈生成的 SYN+ACK。
- checksum 被真实对端接受。
- 在隔离网络环境中完成一次真实三次握手。
- 真实 I/O 适配层不包含 TCP 状态判断。

### 阶段 15：暴露字节流接口并重新承载 HTTP

目的：把自制 TCP 从“会处理报文”升级成“可以给应用层使用的可靠字节流”。

目标接口表达的能力：

```text
connect
listen
accept
read
write
close
```

不要求完全复制标准库 API，但行为含义应当接近。

HTTP 模块需要从具体的 `TcpStream` 中解耦：

```text
HTTP Parser / Response Builder
            |
       字节读取与写入接口
        /             \
std::net::TcpStream   MiniTcpStream
```

验收标准：

- 同一份 HTTP Parser 不需要知道底层使用哪一种 TCP。
- 一个真实客户端可以通过自制 TCP 请求 `/`。
- 大响应被拆成多个 segment 后仍能正确读取。
- 在这一阶段之后，才可以说“HTTP 跑在自制 TCP 上”。

### 阶段 16：在自制 TCP 上恢复 SSE 和 WebSocket

目的：验证长连接和双向帧协议对自制 TCP 的要求。

验收标准：

- SSE 可以连续推送多个事件。
- WebSocket 可以完成 HTTP Upgrade。
- WebSocket Echo 可以在同一条自制 TCP 连接上持续工作。
- TCP 分段边界不会被误认为 HTTP、SSE 或 WebSocket 消息边界。

### 阶段 17：TLS、HTTPS 和 WSS

这个阶段拆成三步。

第一步：观察真实 TLS。

- TLS Record。
- ClientHello 和 ServerHello。
- Certificate。
- key exchange。
- Finished。
- Application Data。

第二步：实现教学版安全通道。

- 手动设计最小 Record。
- nonce、认证标签和防篡改概念。
- 使用受控测试向量理解密钥派生。
- 该实现只用于学习，不声称兼容或安全。

第三步：尝试最小 TLS 1.3 互操作。

- TLS Record 和 Handshake 编解码由自己写。
- 密钥状态机由自己写。
- 密码学原语优先使用经过验证的实现。
- 不把“少用第三方协议库”误解成“生产环境也要自己发明密码学”。

验收标准：

- 能解释 TLS 为什么不等于非对称加密全部数据。
- 能区分 HTTP、HTTPS、WebSocket 和 WSS 的层次。
- 教学版通道能发现密文被修改。
- 互操作目标成功后，HTTP 成为 HTTPS，WebSocket 成为 WSS。

### 阶段 18：TCP 拥塞控制选修

学习内容：

- congestion window。
- slow start。
- congestion avoidance。
- duplicate ACK。
- fast retransmit。
- fast recovery。
- RTT 估计和自适应 RTO。

这一阶段不阻塞本地教学栈和 HTTP 集成，但没有合理拥塞控制的自制 TCP 不应直接用于公网通信。

## 7. 第 8 阶段状态机的结构设计

原来的 `TcpEndpoint` 只有：

```text
send_next
recv_next
```

新版从一开始使用更接近 TCP 术语的三个变量：

```text
snd_una = 最早没有被确认的序列号
snd_nxt = 下一个要发送的序列号
rcv_nxt = 下一个期望接收的序列号
```

例如客户端初始序列号是 100：

```text
发送 SYN 前：snd_una=100, snd_nxt=100
发送 SYN 后：snd_una=100, snd_nxt=101
收到 ack=101：snd_una=101, snd_nxt=101
发送 hello 后：snd_una=101, snd_nxt=106
收到 ack=106：snd_una=106, snd_nxt=106
```

只有同时保存 `snd_una` 和 `snd_nxt`，才能知道：

```text
[snd_una, snd_nxt)
```

这一段序列号对应的数据已经发送，但还没有全部被确认。

推荐初始文件结构：

```text
src/mini_tcp/
  mod.rs
  segment.rs
  state.rs
  endpoint.rs
  simulation.rs
```

进入重传阶段后再增加：

```text
src/mini_tcp/
  event.rs
  action.rs
  timer.rs
  send_buffer.rs
  recv_buffer.rs
  network.rs
```

不在第一天一次创建所有抽象。

## 8. 课程文档的新格式

以前的逐行完整代码适合刚开始学习 Rust 的阶段。进入 TCP 后，课程改成三级提示，但仍保留必要注释。

### 第一级：任务和验收

先提供：

- 本节目标。
- 输入和输出。
- 必须通过的测试。
- 预期状态变化或报文日志。
- 明确不做的功能。

学习者先自己设计。

### 第二级：实现提示

卡住后再查看：

- 建议的数据结构。
- 关键 Rust 语法。
- 协议公式。
- 一小段核心伪代码。
- 常见错误。

### 第三级：完整参考实现

最后才提供：

- 带注释的完整代码。
- 每个模块的职责说明。
- 为什么选择这种实现。
- 与真实协议的差距。

代码注释重点解释：

- 不熟悉的 Rust 语法。
- 字节和位运算。
- 状态为什么这样变化。
- 协议规定背后的作用。

不重复解释一眼就能看懂的赋值语句。

## 9. 每个阶段统一验收方式

每个阶段必须同时拥有四类证据：

1. 正常路径：正确输入能够工作。
2. 异常路径：截断、错误长度、错误状态不会悄悄成功。
3. 故障实验：主动制造分段、断线、丢包、重复或乱序。
4. 口头解释：学习者可以不看代码解释关键状态和字节。

测试层次：

```text
纯函数单元测试
  -> 单端点状态测试
  -> 双端点模拟网络测试
  -> 真实 TUN 集成测试
  -> HTTP / SSE / WebSocket 端到端测试
```

## 10. 错误处理设计

进入 TCP 阶段后，不再主要依靠 `unwrap` 和静默 `None` 表示所有失败。

错误至少区分：

- 输入字节不足。
- Header 字段非法。
- checksum 不正确。
- 当前状态不接受该事件。
- ACK 超出可接受范围。
- 连接超时。
- 对端重置连接。
- 底层设备读写失败。

状态机遇到异常报文时，要明确产生以下结果之一：

```text
忽略
回复 ACK
回复 RST
记录错误但保持连接
终止连接
```

不能简单地把所有异常都处理成 `None`，否则无法知道它是“协议规定要忽略”，还是“代码忘了实现”。

## 11. 路线推进原则

- 一次只增加一个新的困难来源。
- 先确定行为，再写编码和 I/O。
- 先用确定性模拟测试，再接真实网络。
- 协议核心不依赖终端打印、线程或真实时钟。
- 字节编解码不负责状态变化。
- TUN 适配层不负责 TCP 决策。
- HTTP 不直接依赖自制 TCP 的内部结构。
- 每篇文档都明确“完成了什么”和“还没有什么”。

## 12. 下一步文档改造顺序

设计确认后，按以下顺序调整现有项目文档：

1. 更新 `docs/00-roadmap.md`，保留并标记已完成阶段，替换已经过时的后续规划。
2. 更新 `README.md` 的推荐阅读顺序和最终目标。
3. 新增 `docs/08-http-client-and-response-parser.md`。
4. 将当前 `docs/08-mini-tcp-state-machine.md` 重构并改名为 `docs/09-tcp-state-machine-and-sequence-space.md`。
5. 第 09 篇完成并通过验收后，再生成真实 TCP Header 阶段文档。

不会一次生成后面所有阶段的完整参考代码。后续课程会根据前一阶段实际实现和疑问逐篇生成，避免路线和真实学习进度再次脱节。
