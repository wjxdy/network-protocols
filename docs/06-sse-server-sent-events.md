# 第 5 阶段：实现 SSE Server-Sent Events

这一阶段我们开始实现第一个建立在 HTTP 之上的“新协议”：

```text
SSE: Server-Sent Events
```

SSE 的核心思想是：

```text
客户端发起一个普通 HTTP GET 请求
服务端返回一个特殊 Content-Type
服务端不立刻关闭连接
服务端持续往这个连接里写事件数据
```

它的分层关系是：

```text
TCP
  ↓
HTTP
  ↓
SSE
```

所以你前面写的 HTTP Server 基础会继续用上。

## 这一阶段的目标

完成后，你会实现：

- `GET /events`
- SSE 响应头
- SSE 事件格式
- 每秒向客户端推送一条消息
- 用 `curl -N` 实时观察事件流
- 理解为什么 SSE 会卡住单线程服务器
- 引入最简单的并发：一个连接一个线程
- 处理客户端断开连接

## 第 1 步：理解普通 HTTP 和 SSE 的区别

之前我们的 HTTP 响应是这样的：

```text
客户端请求
  ↓
服务端生成完整响应
  ↓
服务端写回响应
  ↓
本次请求结束
```

比如：

```http
GET /hello HTTP/1.1
Host: 127.0.0.1:8080

```

服务端返回：

```http
HTTP/1.1 200 OK
Content-Length: 5
Content-Type: text/plain

hello
```

SSE 不一样。

SSE 是：

```text
客户端请求 /events
  ↓
服务端返回响应头
  ↓
服务端保持连接不断开
  ↓
服务端一条一条推送事件
```

服务端返回的不是一次性 body，而是一条持续不断的事件流。

## 第 2 步：SSE 的响应头

SSE 最重要的响应头是：

```http
Content-Type: text/event-stream
```

一个最小 SSE 响应头可以是：

```http
HTTP/1.1 200 OK
Content-Type: text/event-stream
Cache-Control: no-cache
Connection: keep-alive

```

注意这里没有 `Content-Length`。

为什么？

因为 SSE 是长连接，服务端一开始不知道总共会发送多少数据。

普通响应：

```text
我知道 body 总长度，所以可以写 Content-Length
```

SSE 响应：

```text
我会一直发送事件，不知道最终长度，所以不写 Content-Length
```

## 第 3 步：SSE 消息格式

最简单的 SSE 消息：

```text
data: hello

```

注意：最后有一个空行。

也就是说真实字符串应该是：

```rust
"data: hello\n\n"
```

也可以写事件名：

```text
event: ping
data: hello

```

也可以写 id：

```text
id: 1
event: tick
data: hello

```

这一阶段我们先实现最简单的：

```text
data: tick 1

data: tick 2

data: tick 3

```

## 第 4 步：为什么 curl 要加 `-N`

测试普通 HTTP：

```bash
curl --noproxy '*' -v http://127.0.0.1:8080/
```

测试 SSE 推荐：

```bash
curl --noproxy '*' -N http://127.0.0.1:8080/events
```

`-N` 的意思是：

```text
不要缓冲输出，收到一点就立刻显示一点
```

SSE 是流式输出，如果不加 `-N`，curl 可能会先攒一会儿再显示，看起来就不像实时推送。

## 第 5 步：先写一个 SSE 响应头函数

我们先在 `src/http/server.rs` 里加一个函数。

这个函数只负责写 SSE 的 HTTP 响应头。

```rust
// 向客户端写入 SSE 响应头。
fn write_sse_headers(stream: &mut TcpStream) -> std::io::Result<()> {
    // SSE 是 HTTP 响应，所以第一行仍然是 HTTP 状态行。
    //
    // Content-Type: text/event-stream
    // 告诉客户端：这是 SSE 事件流。
    //
    // Cache-Control: no-cache
    // 告诉客户端不要缓存事件流。
    //
    // Connection: keep-alive
    // 表示希望连接保持打开。
    //
    // 最后的 \r\n\r\n 表示 headers 结束。
    let headers = concat!(
        "HTTP/1.1 200 OK\r\n",
        "Content-Type: text/event-stream\r\n",
        "Cache-Control: no-cache\r\n",
        "Connection: keep-alive\r\n",
        "\r\n"
    );

    // 把响应头写到 TCP 连接里。
    stream.write_all(headers.as_bytes())
}
```

这里第一次用了：

```rust
std::io::Result<()>
```

意思是：

```text
成功：Ok(())
失败：Err(...)
```

如果客户端断开了，`write_all` 就可能返回错误。

## 第 6 步：写一个 SSE 事件格式函数

我们写一个小函数，把普通文本变成 SSE 格式。

```rust
// 把一段文本编码成 SSE data 事件。
fn sse_data(message: &str) -> String {
    // SSE 的最小格式是：
    //
    // data: 消息内容
    //
    //
    // 注意最后要有两个 \n。
    format!("data: {}\n\n", message)
}
```

比如：

```rust
let event = sse_data("hello");
```

得到：

```text
data: hello

```

## 第 7 步：实现 `handle_sse`

现在实现真正的 SSE 处理函数。

它会：

```text
1. 写 SSE 响应头	
2. 每秒写一条事件
3. 如果客户端断开，就停止循环
```

代码：

```rust
// 导入 sleep 和 Duration。
// sleep 用来让当前线程暂停一段时间。
// Duration 用来表示时间长度。
use std::thread::sleep;
use std::time::Duration;

// 处理 SSE 连接。
fn handle_sse(mut stream: TcpStream) {
    // 先写 SSE 响应头。
    //
    // 如果这里失败，说明连接可能已经断开，直接返回。
    if write_sse_headers(&mut stream).is_err() {
        return;
    }

    // 从 1 开始计数。
    let mut count = 1;

    // 不断发送事件。
    loop {
        // 构造消息文本。
        let message = format!("tick {}", count);

        // 编码成 SSE 格式。
        let event = sse_data(&message);

        // 写入 TCP 连接。
        //
        // 如果写失败，通常说明客户端断开了。
        if stream.write_all(event.as_bytes()).is_err() {
            println!("SSE client disconnected");
            break;
        }

        // 尽量立刻把数据刷出去。
        //
        // 有些写入会先进入缓冲区，flush 可以提示系统尽快发送。
        if stream.flush().is_err() {
            println!("SSE flush failed");
            break;
        }

        // 计数加 1。
        count += 1;

        // 暂停 1 秒。
        sleep(Duration::from_secs(1));
    }
}
```

这里有一个重要点：

```rust
handle_sse(mut stream: TcpStream)
```

它拿走了这个连接。

因为 SSE 会持续使用这个连接，不会马上返回。

## 第 8 步：在 `handle_connection` 里识别 `/events`

之前你的 `handle_connection` 大概是：

```rust
fn handle_connection(mut stream: TcpStream) {
    let Some(raw_request) = read_http_request(&mut stream) else {
        return;
    };

    let request = parse_request(&raw_request).unwrap();
    let response = route(&request);
    let response_bytes = response.to_http_bytes();

    stream.write_all(&response_bytes).unwrap();
}
```

现在要加一个特殊分支：

```rust
// 处理一个 TCP 连接。
fn handle_connection(mut stream: TcpStream) {
    // 先读取完整 HTTP 请求。
    let Some(raw_request) = read_http_request(&mut stream) else {
        return;
    };

    // 打印原始请求，方便调试。
    println!(
        "Raw request:\n{}",
        String::from_utf8_lossy(&raw_request)
    );

    // 解析请求。
    let request = parse_request(&raw_request).unwrap();

    // 如果访问 GET /events，就进入 SSE 处理。
    //
    // 注意：handle_sse 会长期占用这个连接。
    if request.method == "GET" && request.path == "/events" {
        handle_sse(stream);
        return;
    }

    // 普通 HTTP 请求仍然走原来的 route。
    let response = route(&request);
    let response_bytes = response.to_http_bytes();

    stream.write_all(&response_bytes).unwrap();
}
```

这里为什么 `handle_sse(stream)` 后要 `return`？

因为这个连接已经交给 SSE 处理了。

后面不能再继续写普通 HTTP 响应。

## 第 9 步：测试 SSE

运行服务：

```bash
cargo run
```

另开一个终端：

```bash
curl --noproxy '*' -N http://127.0.0.1:8080/events
```

你应该会看到：

```text
data: tick 1

data: tick 2

data: tick 3

```

它会每秒继续输出。

按 `Ctrl+C` 可以断开 curl。

服务端可能会打印：

```text
SSE client disconnected
```

这说明客户端断开后，服务端写入失败，然后退出 SSE 循环。

## 第 10 步：观察单线程阻塞问题

如果你的 server 还是这样：

```rust
for stream in listener.incoming() {
    let stream = stream.unwrap();
    handle_connection(stream);
}
```

那么有一个现象：

```text
一个客户端连接 /events 后
handle_connection 会进入 handle_sse
handle_sse 会一直循环
for 循环回不到下一轮
其他请求就暂时进不来
```

测试方法：

终端 A：

```bash
curl --noproxy '*' -N http://127.0.0.1:8080/events
```

不要关。

终端 B：

```bash
curl --noproxy '*' -v http://127.0.0.1:8080/
```

如果你的服务器是单线程，终端 B 可能会卡住。

这就是并发问题出现的原因。

不是因为并发很酷，而是因为：

```text
长连接会占住当前线程
```

## 第 11 步：引入最简单的并发

我们先不用线程池，也不用 async。

只做最简单的：

```text
每来一个连接，就开一个新线程处理
```

把 `serve` 从：

```rust
pub fn serve(addr: &str) {
    let listener = TcpListener::bind(addr).unwrap();

    println!("Server listening on http://{}", addr);

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(stream);
    }
}
```

改成：

```rust
// 启动 HTTP Server。
pub fn serve(addr: &str) {
    // 监听地址。
    let listener = TcpListener::bind(addr).unwrap();

    println!("Server listening on http://{}", addr);

    // 不断接受连接。
    for stream in listener.incoming() {
        // 取出 TcpStream。
        let stream = stream.unwrap();

        // 每个连接启动一个新线程。
        //
        // move 的意思是：
        // 把 stream 的所有权移动进这个新线程。
        std::thread::spawn(move || {
            handle_connection(stream);
        });
    }
}
```

这里的：

```rust
std::thread::spawn(move || {
    handle_connection(stream);
});
```

可以先这样理解：

```text
创建一个新线程
在线程里执行 handle_connection(stream)
主线程立刻回去继续 accept 下一个连接
```

`move` 是必须的，因为 `stream` 要交给新线程使用。

## 第 12 步：再次测试并发

终端 A：

```bash
curl --noproxy '*' -N http://127.0.0.1:8080/events
```

保持不关。

终端 B：

```bash
curl --noproxy '*' -v http://127.0.0.1:8080/
```

这次终端 B 应该能正常返回。

因为：

```text
/events 在一个线程里持续运行
/ 在另一个线程里处理
主线程继续接受新连接
```

## 第 13 步：完整的 `server.rs` 参考

下面是一份完整的 `src/http/server.rs` 参考。

你可以对照修改，不建议一口气复制。最好一段一段敲。

```rust
// Read 用来从 TCP 连接读取字节。
// Write 用来把响应字节写回 TCP 连接。
use std::io::{Read, Write};

// TcpListener 用来监听端口。
// TcpStream 表示一个已经建立的 TCP 连接。
use std::net::{TcpListener, TcpStream};

// sleep 用来让当前线程暂停一段时间。
use std::thread::sleep;

// Duration 用来表示时间长度。
use std::time::Duration;

// 导入 parser 里的函数。
use super::parser::{
    find_header_end,
    parse_content_length_from_head,
    parse_request,
};

// 导入 Request 和 Response。
use super::{Request, Response};

// 从 TCP 连接读取一个完整 HTTP 请求。
fn read_http_request(stream: &mut TcpStream) -> Option<Vec<u8>> {
    // 保存所有已经读到的字节。
    let mut buffer = Vec::new();

    // 每次临时读取 1024 字节。
    let mut temp = [0; 1024];

    // 完整请求应该有多少字节。
    let mut expected_total: Option<usize> = None;

    loop {
        // 从 TCP 连接读取字节。
        let bytes_read = stream.read(&mut temp).unwrap();

        // 读到 0 表示对方关闭连接。
        if bytes_read == 0 {
            break;
        }

        // 把这次读到的内容追加到总 buffer。
        buffer.extend_from_slice(&temp[..bytes_read]);

        // 如果还不知道完整长度，就尝试找到 headers 结束位置。
        if expected_total.is_none() {
            if let Some(header_end) = find_header_end(&buffer) {
                // 注意这里要取 buffer[..header_end]。
                let head_text = String::from_utf8_lossy(&buffer[..header_end]);

                // 解析 Content-Length，没有就当作 0。
                let content_length =
                    parse_content_length_from_head(&head_text).unwrap_or(0);

                // body 从 \r\n\r\n 后面开始，所以加 4。
                let body_start = header_end + 4;

                // 完整请求总长度。
                expected_total = Some(body_start + content_length);
            }
        }

        // 如果已经读够完整请求，就停止。
        if let Some(total) = expected_total {
            if buffer.len() >= total {
                break;
            }
        }
    }

    if buffer.is_empty() {
        None
    } else {
        Some(buffer)
    }
}

// 向客户端写入 SSE 响应头。
fn write_sse_headers(stream: &mut TcpStream) -> std::io::Result<()> {
    let headers = concat!(
        "HTTP/1.1 200 OK\r\n",
        "Content-Type: text/event-stream\r\n",
        "Cache-Control: no-cache\r\n",
        "Connection: keep-alive\r\n",
        "\r\n"
    );

    stream.write_all(headers.as_bytes())
}

// 把文本编码成 SSE data 事件。
fn sse_data(message: &str) -> String {
    format!("data: {}\n\n", message)
}

// 处理 SSE 连接。
fn handle_sse(mut stream: TcpStream) {
    // 先写 SSE 响应头。
    if write_sse_headers(&mut stream).is_err() {
        return;
    }

    let mut count = 1;

    loop {
        let message = format!("tick {}", count);
        let event = sse_data(&message);

        if stream.write_all(event.as_bytes()).is_err() {
            println!("SSE client disconnected");
            break;
        }

        if stream.flush().is_err() {
            println!("SSE flush failed");
            break;
        }

        count += 1;

        sleep(Duration::from_secs(1));
    }
}

// 根据请求生成普通 HTTP 响应。
fn route(request: &Request) -> Response {
    println!(
        "method={}, path={}, version={}",
        request.method, request.path, request.version
    );

    if let Some(host) = request.header("Host") {
        println!("Host header: {}", host);
    }

    println!("body length: {}", request.body.len());

    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") => {
            let body = b"<h1>Home</h1><p>SSE server is running.</p>".to_vec();
            let mut response = Response::new(200, "OK", body);
            response.set_header("Content-Type", "text/html; charset=utf-8");
            response
        }
        ("GET", "/hello") => {
            let body = b"Hello from Rust HTTP server".to_vec();
            let mut response = Response::new(200, "OK", body);
            response.set_header("Content-Type", "text/plain; charset=utf-8");
            response
        }
        ("POST", "/echo") => {
            let mut response = Response::new(200, "OK", request.body.clone());
            response.set_header("Content-Type", "text/plain; charset=utf-8");
            response
        }
        ("POST", "/submit") => {
            let submitted = request.body_text();
            let html = format!("<h1>Submitted</h1><pre>{}</pre>", submitted);
            let mut response = Response::new(200, "OK", html.into_bytes());
            response.set_header("Content-Type", "text/html; charset=utf-8");
            response
        }
        _ => {
            let body = b"Not Found".to_vec();
            let mut response = Response::new(404, "Not Found", body);
            response.set_header("Content-Type", "text/plain; charset=utf-8");
            response
        }
    }
}

// 处理一个 TCP 连接。
fn handle_connection(mut stream: TcpStream) {
    let Some(raw_request) = read_http_request(&mut stream) else {
        return;
    };

    println!(
        "Raw request:\n{}",
        String::from_utf8_lossy(&raw_request)
    );

    let request = parse_request(&raw_request).unwrap();

    if request.method == "GET" && request.path == "/events" {
        handle_sse(stream);
        return;
    }

    let response = route(&request);
    let response_bytes = response.to_http_bytes();

    stream.write_all(&response_bytes).unwrap();
}

// 启动 HTTP Server。
pub fn serve(addr: &str) {
    let listener = TcpListener::bind(addr).unwrap();

    println!("Server listening on http://{}", addr);

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        std::thread::spawn(move || {
            handle_connection(stream);
        });
    }
}
```

## 第 14 步：你应该理解的问题

完成后，你应该能回答：

1. SSE 为什么仍然是 HTTP？
2. `Content-Type: text/event-stream` 的作用是什么？
3. SSE 为什么不写 `Content-Length`？
4. `data: hello\n\n` 里最后两个换行为什么重要？
5. 为什么单线程服务器会被 `/events` 卡住？
6. `std::thread::spawn(move || { ... })` 大概做了什么？
7. 客户端断开后，服务端怎么知道？

## 当前版本的局限

这个版本能跑 SSE，但仍然是教学版：

- 每个连接一个线程，不适合大量连接
- 没有线程池
- 没有 async IO
- 没有支持 SSE 的 `event:` 和 `id:`
- 没有心跳注释 `: ping`
- 没有断线重连机制
- 没有广播给多个客户端

下一阶段可以继续做：

```text
WebSocket
```

因为你现在已经理解了：

```text
HTTP 普通响应
HTTP POST body
HTTP 长连接 SSE
```

接下来就可以学习：

```text
HTTP Upgrade -> WebSocket frame
```

