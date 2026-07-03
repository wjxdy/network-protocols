# 第 2 阶段：把 HTTP Server 拆成 Request 和 Response

上一阶段你已经写出了一个最小 HTTP Server。

它能做到：

- 监听 TCP 端口
- 读取 HTTP 请求
- 解析请求行
- 根据 path 返回不同内容
- 返回 `200 OK` 或 `404 Not Found`

这一阶段我们不急着做 SSE 或 WebSocket。

我们先把 HTTP Server 的结构打稳，让它更像一个真正的协议实现。

## 这一阶段的目标

完成后，你会拥有：

- `Request` 结构体：保存 method、path、version、headers
- `Header` 结构体：保存一个 HTTP header
- `Response` 结构体：保存 status、headers、body
- `parse_request` 函数：解析请求行和 headers
- `Response::to_http_bytes` 方法：把响应对象编码成 HTTP 字节
- 更清楚的 `handle_connection`

这一阶段暂时还不处理 request body，也暂时不支持并发。

## 为什么要拆结构

上一版里，我们直接这样拼响应：

```rust
let response = format!(
    "{}\r\nContent-Length: {}\r\nContent-Type: {}\r\n\r\n{}",
    status_line,
    body.as_bytes().len(),
    content_type,
    body
);
```

这能跑，但有一个问题：所有 HTTP 细节都散落在字符串里。

如果后面要支持：

- 多个 headers
- 不同状态码
- SSE 长连接
- WebSocket Upgrade
- HTTP Client

直接拼字符串会越来越乱。

所以我们要开始把协议概念变成 Rust 结构体：

```text
HTTP 请求  -> Request
HTTP 响应  -> Response
HTTP 头    -> Header
```

这一步很重要。协议实现不是只写字符串，而是要把协议里的概念建模出来。

## 第 1 步：先理解 HTTP 请求结构

浏览器发来的 HTTP 请求大概长这样：

```http
GET /hello HTTP/1.1
Host: 127.0.0.1:8080
User-Agent: curl/8.0.0
Accept: */*

```

注意它分成两部分：

```text
请求行
headers
空行
可选 body
```

目前我们只解析：

```text
请求行 + headers
```

先不解析 body。

## 第 2 步：定义 Header

HTTP header 是一行键值对：

```http
Host: 127.0.0.1:8080
```

左边是 name，右边是 value。

在 Rust 里可以这样建模：

```rust
// Header 表示一行 HTTP header。
// 例如：Host: 127.0.0.1:8080
#[derive(Debug)]
struct Header {
    // header 名字，比如 Host、User-Agent、Content-Type。
    name: String,

    // header 的值，比如 127.0.0.1:8080。
    value: String,
}
```

`#[derive(Debug)]` 的作用还是一样：让它可以被 `println!("{:?}", header)` 打印出来。

## 第 3 步：升级 Request

上一阶段的 `Request` 只有：

```rust
method
path
version
```

这一阶段加上：

```rust
headers
```

代码：

```rust
// Request 表示一个 HTTP 请求。
#[derive(Debug)]
struct Request {
    // HTTP 方法，比如 GET、POST。
    method: String,

    // 请求路径，比如 /、/hello。
    path: String,

    // HTTP 版本，比如 HTTP/1.1。
    version: String,

    // 请求 headers。
    // Vec<Header> 表示一个可以增长的 Header 列表。
    headers: Vec<Header>,
}
```

这里出现了：

```rust
Vec<Header>
```

可以理解成：

```text
一个装 Header 的列表
```

因为一个 HTTP 请求里可以有很多 header。

## 第 4 步：解析 header 行

一行 header 长这样：

```http
Host: 127.0.0.1:8080
```

我们想把它拆成：

```text
name  = Host
value = 127.0.0.1:8080
```

代码：

```rust
// 解析一行 HTTP header。
// 输入例子："Host: 127.0.0.1:8080"
// 输出例子：Some(Header { name: "Host", value: "127.0.0.1:8080" })
fn parse_header_line(line: &str) -> Option<Header> {
    // split_once(':') 会按第一个冒号切开字符串。
    // 如果 line 里没有冒号，就返回 None。
    let (name, value) = line.split_once(':')?;

    // trim() 用来去掉前后的空格。
    // HTTP header 的冒号后面通常会有一个空格。
    let name = name.trim().to_string();
    let value = value.trim().to_string();

    // 如果 header 名字为空，说明格式不对。
    if name.is_empty() {
        return None;
    }

    // 解析成功，返回 Header。
    Some(Header { name, value })
}
```

这里的核心是：

```rust
line.split_once(':')
```

它只按第一个冒号切。

为什么不是直接 `split(':')`？

因为 header value 里也可能有冒号，比如：

```http
Host: 127.0.0.1:8080
```

如果按所有冒号切，会把 `127.0.0.1:8080` 也切坏。

## 第 5 步：解析完整 Request

现在我们升级 `parse_request`。

目标：

```text
1. 取第一行作为 request line
2. 解析 method/path/version
3. 后面的每一行都尝试解析成 Header
4. 遇到空行停止
```

代码：

```rust
// 解析完整 HTTP 请求。
fn parse_request(request_text: &str) -> Option<Request> {
    // lines() 返回一个按行读取的迭代器。
    let mut lines = request_text.lines();

    // 第一行是 request line，例如："GET /hello HTTP/1.1"。
    let request_line = lines.next()?;

    // 按空白字符切分 request line。
    let mut parts = request_line.split_whitespace();

    // 第一段是 method。
    let method = parts.next()?.to_string();

    // 第二段是 path。
    let path = parts.next()?.to_string();

    // 第三段是 version。
    let version = parts.next()?.to_string();

    // 创建一个空 Vec，用来保存 headers。
    let mut headers = Vec::new();

    // 继续读取后面的每一行。
    for line in lines {
        // 空行表示 headers 结束。
        if line.trim().is_empty() {
            break;
        }

        // 尝试把这一行解析成 Header。
        let header = parse_header_line(line)?;

        // 把解析出的 header 放进 headers 列表。
        headers.push(header);
    }

    // 构造 Request。
    Some(Request {
        method,
        path,
        version,
        headers,
    })
}
```

这里有一个新东西：

```rust
headers.push(header);
```

意思是：往 `Vec` 列表末尾加入一个元素。

## 第 6 步：给 Request 增加查找 header 的方法

后面做 SSE、WebSocket 时，我们经常需要找某个 header。

比如 WebSocket 会看：

```http
Upgrade: websocket
Sec-WebSocket-Key: ...
```

所以我们给 `Request` 加一个方法：

```rust
impl Request {
    // 根据 header 名字查找对应的值。
    // 返回 Option<&str>，表示可能找到，也可能找不到。
    fn header(&self, name: &str) -> Option<&str> {
        // 遍历所有 headers。
        for header in &self.headers {
            // HTTP header 名字大小写不敏感。
            // 所以 Host 和 host 应该被认为是同一个 header。
            if header.name.eq_ignore_ascii_case(name) {
                // 找到了，就返回 value 的字符串切片。
                return Some(header.value.as_str());
            }
        }

        // 全部找完都没找到，返回 None。
        None
    }
}
```

这里的：

```rust
&self.headers
```

表示借用 headers 列表，不拿走它。

如果写成：

```rust
for header in self.headers
```

就会尝试把 headers 从 Request 里拿走，这通常不是我们想要的。

## 第 7 步：定义 Response

上一阶段我们用几个变量拼响应：

```rust
status_line
content_type
body
```

现在改成结构体：

```rust
// Response 表示一个 HTTP 响应。
struct Response {
    // HTTP 状态码，比如 200、404。
    status_code: u16,

    // 状态原因短语，比如 OK、Not Found。
    reason: String,

    // 响应 headers。
    headers: Vec<Header>,

    // 响应 body。
    // 用 Vec<u8> 而不是 String，是因为 HTTP body 本质上是字节。
    body: Vec<u8>,
}
```

为什么 body 用 `Vec<u8>`？

因为 HTTP body 不一定是文本。

它也可能是：

- HTML
- JSON
- 图片
- 文件
- WebSocket 握手前后的数据

所以从协议角度看，body 最准确的类型是“一串字节”。

## 第 8 步：实现 Response 构造函数

我们给 `Response` 写几个辅助方法。

代码：

```rust
impl Response {
    // 创建一个新的 Response。
    fn new(status_code: u16, reason: &str, body: Vec<u8>) -> Response {
        // 先创建 Response，headers 暂时为空。
        let mut response = Response {
            status_code,
            reason: reason.to_string(),
            headers: Vec::new(),
            body,
        };

        // 自动加上 Content-Length。
        // 注意：这里用 response.body.len()，因为 body 本来就是 Vec<u8>。
        let content_length = response.body.len().to_string();
        response.set_header("Content-Length", &content_length);

        // 返回构造好的 Response。
        response
    }

    // 设置一个 header。
    fn set_header(&mut self, name: &str, value: &str) {
        // 先看看是否已经存在同名 header。
        for header in &mut self.headers {
            // header 名字大小写不敏感。
            if header.name.eq_ignore_ascii_case(name) {
                // 如果已经存在，就更新 value。
                header.value = value.to_string();
                return;
            }
        }

        // 如果不存在，就新增一个 Header。
        self.headers.push(Header {
            name: name.to_string(),
            value: value.to_string(),
        });
    }
}
```

这里第一次出现了：

```rust
&mut self.headers
```

意思是：可变地借用 headers 列表。

因为我们可能要修改 header 的 value。

## 第 9 步：把 Response 编码成 HTTP 字节

现在我们有了 `Response` 对象，但网络里最终要发送的是字节。

所以要写一个方法：

```rust
Response -> Vec<u8>
```

代码：

```rust
impl Response {
    // 把 Response 编码成符合 HTTP 格式的字节。
    fn to_http_bytes(&self) -> Vec<u8> {
        // 先拼状态行，例如："HTTP/1.1 200 OK\r\n"。
        let mut response = format!("HTTP/1.1 {} {}\r\n", self.status_code, self.reason);

        // 拼每一个 header。
        for header in &self.headers {
            response.push_str(&format!("{}: {}\r\n", header.name, header.value));
        }

        // 空行表示 headers 结束。
        response.push_str("\r\n");

        // 先把状态行和 headers 转成字节。
        let mut bytes = response.into_bytes();

        // 再把 body 字节追加到后面。
        bytes.extend_from_slice(&self.body);

        // 返回完整 HTTP 响应字节。
        bytes
    }
}
```

注意这里：

```rust
response.into_bytes()
```

会把 `String` 转成 `Vec<u8>`。

然后：

```rust
bytes.extend_from_slice(&self.body);
```

表示把 body 的字节追加到 headers 后面。

## 第 10 步：写路由函数

现在我们可以把“根据 path 返回内容”的逻辑也拆出来。

代码：

```rust
// 根据 Request 构造 Response。
fn route(request: &Request) -> Response {
    // 打印 Host header，帮助你确认 header 解析成功了。
    if let Some(host) = request.header("Host") {
        println!("Host header: {}", host);
    }

    // 根据 path 决定响应内容。
    match request.path.as_str() {
        "/" => {
            // HTML body。
            let body = b"<h1>Home</h1><p>Structured HTTP server.</p>".to_vec();

            // 构造 200 OK 响应。
            let mut response = Response::new(200, "OK", body);

            // 设置 Content-Type。
            response.set_header("Content-Type", "text/html; charset=utf-8");

            // 返回 response。
            response
        }
        "/hello" => {
            // 纯文本 body。
            let body = b"Hello from structured Rust HTTP server".to_vec();

            // 构造响应。
            let mut response = Response::new(200, "OK", body);

            // 设置 Content-Type。
            response.set_header("Content-Type", "text/plain; charset=utf-8");

            // 返回 response。
            response
        }
        _ => {
            // 未知路径返回 404。
            let body = b"Not Found".to_vec();
            let mut response = Response::new(404, "Not Found", body);
            response.set_header("Content-Type", "text/plain; charset=utf-8");
            response
        }
    }
}
```

这里出现了：

```rust
b"...".to_vec()
```

`b"..."` 是字节字符串。

`.to_vec()` 把它变成 `Vec<u8>`。

## 第 11 步：完整代码

现在把这一阶段的代码合在一起。

你可以把 `src/main.rs` 改成下面这样：

```rust
// 从标准库导入 Read 和 Write。
// Read 用来从 TcpStream 读取请求字节。
// Write 用来把响应字节写回 TcpStream。
use std::io::{Read, Write};

// TcpListener 用来监听端口。
// TcpStream 表示一个已经建立好的 TCP 连接。
use std::net::{TcpListener, TcpStream};

// Header 表示一行 HTTP header。
// 例如：Host: 127.0.0.1:8080
#[derive(Debug)]
struct Header {
    // header 名字，比如 Host、User-Agent、Content-Type。
    name: String,

    // header 值，比如 127.0.0.1:8080。
    value: String,
}

// Request 表示一个 HTTP 请求。
#[derive(Debug)]
struct Request {
    // HTTP 方法，比如 GET、POST。
    method: String,

    // 请求路径，比如 /、/hello。
    path: String,

    // HTTP 版本，比如 HTTP/1.1。
    version: String,

    // 请求 headers。
    headers: Vec<Header>,
}

// 给 Request 添加方法。
impl Request {
    // 根据 header 名字查找 header 值。
    fn header(&self, name: &str) -> Option<&str> {
        // 遍历所有 headers。
        for header in &self.headers {
            // HTTP header 名字大小写不敏感。
            if header.name.eq_ignore_ascii_case(name) {
                // 找到后返回 value。
                return Some(header.value.as_str());
            }
        }

        // 没找到返回 None。
        None
    }
}

// Response 表示一个 HTTP 响应。
struct Response {
    // HTTP 状态码，例如 200、404。
    status_code: u16,

    // 状态原因短语，例如 OK、Not Found。
    reason: String,

    // 响应 headers。
    headers: Vec<Header>,

    // 响应 body。
    // 用 Vec<u8> 表示它本质是一串字节。
    body: Vec<u8>,
}

// 给 Response 添加方法。
impl Response {
    // 创建一个新的 Response。
    fn new(status_code: u16, reason: &str, body: Vec<u8>) -> Response {
        // 先创建 Response。
        let mut response = Response {
            status_code,
            reason: reason.to_string(),
            headers: Vec::new(),
            body,
        };

        // 自动设置 Content-Length。
        // HTTP 客户端需要知道 body 有多少字节。
        // 这里先保存到变量里，再传给 set_header，写法更清楚。
        let content_length = response.body.len().to_string();
        response.set_header("Content-Length", &content_length);

        // 返回 Response。
        response
    }

    // 设置 header。
    // 如果同名 header 已存在，就更新。
    // 如果不存在，就新增。
    fn set_header(&mut self, name: &str, value: &str) {
        // 遍历已有 headers，看看有没有同名 header。
        for header in &mut self.headers {
            // header 名字大小写不敏感。
            if header.name.eq_ignore_ascii_case(name) {
                // 找到了就更新 value。
                header.value = value.to_string();
                return;
            }
        }

        // 没找到就新增一个 Header。
        self.headers.push(Header {
            name: name.to_string(),
            value: value.to_string(),
        });
    }

    // 把 Response 转换成真正要写到 TCP 连接里的 HTTP 字节。
    fn to_http_bytes(&self) -> Vec<u8> {
        // 先构造状态行。
        // 例如：HTTP/1.1 200 OK\r\n
        let mut head = format!("HTTP/1.1 {} {}\r\n", self.status_code, self.reason);

        // 把每个 header 拼进去。
        for header in &self.headers {
            // 每个 header 一行，格式是：Name: Value\r\n
            head.push_str(&format!("{}: {}\r\n", header.name, header.value));
        }

        // 空行表示 headers 结束，后面开始是 body。
        head.push_str("\r\n");

        // 把状态行和 headers 转成字节。
        let mut bytes = head.into_bytes();

        // 把 body 字节追加到后面。
        bytes.extend_from_slice(&self.body);

        // 返回完整响应字节。
        bytes
    }
}

// 解析一行 HTTP header。
fn parse_header_line(line: &str) -> Option<Header> {
    // 按第一个冒号切开。
    let (name, value) = line.split_once(':')?;

    // 去掉 name 和 value 两边的空格。
    let name = name.trim().to_string();
    let value = value.trim().to_string();

    // header 名字不能为空。
    if name.is_empty() {
        return None;
    }

    // 返回 Header。
    Some(Header { name, value })
}

// 解析 HTTP 请求。
fn parse_request(request_text: &str) -> Option<Request> {
    // 按行读取 HTTP 请求文本。
    let mut lines = request_text.lines();

    // 第一行是请求行，例如：GET /hello HTTP/1.1
    let request_line = lines.next()?;

    // 按空白切分请求行。
    let mut parts = request_line.split_whitespace();

    // 第一段是 method。
    let method = parts.next()?.to_string();

    // 第二段是 path。
    let path = parts.next()?.to_string();

    // 第三段是 HTTP version。
    let version = parts.next()?.to_string();

    // 创建空 headers 列表。
    let mut headers = Vec::new();

    // 继续解析后面的 header 行。
    for line in lines {
        // 空行表示 headers 结束。
        if line.trim().is_empty() {
            break;
        }

        // 解析当前 header 行。
        let header = parse_header_line(line)?;

        // 把 header 放进列表。
        headers.push(header);
    }

    // 返回 Request。
    Some(Request {
        method,
        path,
        version,
        headers,
    })
}

// 根据请求生成响应。
fn route(request: &Request) -> Response {
    // 打印 method、path、version，帮助你观察解析结果。
    println!(
        "method={}, path={}, version={}",
        request.method, request.path, request.version
    );

    // 尝试读取 Host header。
    if let Some(host) = request.header("Host") {
        println!("Host header: {}", host);
    }

    // 根据不同 path 返回不同 Response。
    match request.path.as_str() {
        "/" => {
            // HTML body。
            let body = b"<h1>Home</h1><p>Structured HTTP server.</p>".to_vec();

            // 创建 200 OK 响应。
            let mut response = Response::new(200, "OK", body);

            // 设置 Content-Type。
            response.set_header("Content-Type", "text/html; charset=utf-8");

            // 返回响应。
            response
        }
        "/hello" => {
            // 纯文本 body。
            let body = b"Hello from structured Rust HTTP server".to_vec();

            // 创建 200 OK 响应。
            let mut response = Response::new(200, "OK", body);

            // 设置 Content-Type。
            response.set_header("Content-Type", "text/plain; charset=utf-8");

            // 返回响应。
            response
        }
        _ => {
            // 未知路径返回 404。
            let body = b"Not Found".to_vec();

            // 创建 404 Not Found 响应。
            let mut response = Response::new(404, "Not Found", body);

            // 设置 Content-Type。
            response.set_header("Content-Type", "text/plain; charset=utf-8");

            // 返回响应。
            response
        }
    }
}

// 处理单个 TCP 连接。
fn handle_connection(mut stream: TcpStream) {
    // 创建 4096 字节缓冲区。
    // 这一阶段比上一阶段稍微加大一点。
    let mut buffer = [0; 4096];

    // 从 TCP 连接读取请求字节。
    let bytes_read = stream.read(&mut buffer).unwrap();

    // 如果读到 0 字节，通常表示连接已经关闭。
    if bytes_read == 0 {
        return;
    }

    // 只取实际读到的字节，并按文本显示。
    let request_text = String::from_utf8_lossy(&buffer[..bytes_read]);

    // 打印原始请求，方便调试。
    println!("Raw request:\n{}", request_text);

    // 解析请求。
    let request = parse_request(&request_text).unwrap();

    // 根据请求生成响应。
    let response = route(&request);

    // 把 Response 编码成 HTTP 字节。
    let response_bytes = response.to_http_bytes();

    // 写回客户端。
    stream.write_all(&response_bytes).unwrap();
}

// 程序入口。
fn main() {
    // 监听本机 8080 端口。
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    // 打印启动信息。
    println!("Server listening on http://127.0.0.1:8080");

    // 不断接受新的 TCP 连接。
    for stream in listener.incoming() {
        // 取出 TcpStream。
        let stream = stream.unwrap();

        // 处理这个连接。
        handle_connection(stream);
    }
}
```

## 第 12 步：运行测试

运行服务：

```bash
cargo run
```

另开一个终端：

```bash
curl -v http://127.0.0.1:8080/
```

你应该能看到响应：

```html
<h1>Home</h1><p>Structured HTTP server.</p>
```

测试 `/hello`：

```bash
curl -v http://127.0.0.1:8080/hello
```

测试 404：

```bash
curl -v http://127.0.0.1:8080/not-found
```

你应该能看到：

```text
< HTTP/1.1 404 Not Found
```

## 第 13 步：观察 headers

使用：

```bash
curl -v http://127.0.0.1:8080/hello
```

你会看到 curl 发出的请求 headers：

```text
> GET /hello HTTP/1.1
> Host: 127.0.0.1:8080
> User-Agent: curl/...
> Accept: */*
```

你的服务端会打印：

```text
Host header: 127.0.0.1:8080
```

说明你已经成功解析了 headers。

## 这一阶段你应该理解的问题

完成后，你应该能回答：

1. 为什么 `Request` 里要有 `headers: Vec<Header>`？
2. 为什么解析 header 要用 `split_once(':')`，而不是随便按冒号全部切开？
3. 为什么 `Response.body` 用 `Vec<u8>`，而不是 `String`？
4. `Response::to_http_bytes` 做了什么？
5. `Content-Length` 是在哪里自动设置的？
6. `request.header("Host")` 为什么返回 `Option<&str>`？

## 当前版本的局限

这个版本比上一版结构更清楚，但仍然有局限：

- 还是只读一次 TCP 数据
- 请求超过 4096 字节会读不完整
- 没有处理 request body
- 没有支持 POST
- 没有更严格的 HTTP 错误处理
- 没有并发
- 所有代码还在 `main.rs`

下一阶段可以继续做：

- 支持读取完整 HTTP request
- 支持 `Content-Length` 请求体
- 支持 `POST`
- 把代码拆到 `src/http/` 目录
