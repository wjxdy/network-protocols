# 第 3 阶段：支持 HTTP Request Body 和 POST

上一阶段你已经把 HTTP Server 拆出了：

- `Header`
- `Request`
- `Response`
- `parse_request`
- `route`

这一阶段我们继续升级 HTTP Server，让它支持请求体，也就是 request body。

完成这一阶段后，你可以用 `curl` 发送 POST 请求：

```bash
curl --noproxy '*' -v -X POST -d "hello from client" http://127.0.0.1:8080/echo
```

服务端会读到 body，并把它返回给客户端。

## 这一阶段为什么重要

前面我们只处理了这种请求：

```http
GET /hello HTTP/1.1
Host: 127.0.0.1:8080

```

这种请求通常没有 body。

但很多真实请求是有 body 的，比如：

```http
POST /echo HTTP/1.1
Host: 127.0.0.1:8080
Content-Length: 17
Content-Type: text/plain

hello from client
```

注意这里多了一个很重要的 header：

```http
Content-Length: 17
```

它告诉服务端：后面的 body 有 17 个字节。

## 这一阶段的目标

你会实现：

- `Request.body: Vec<u8>`
- 读取完整 HTTP 请求
- 根据 `Content-Length` 判断 body 是否读完
- 支持 `POST /echo`
- 支持 `POST /submit`
- 理解为什么一次 `stream.read` 不一定读完整请求

## 第 1 步：理解 GET 和 POST 的区别

简单理解：

```text
GET  通常用来获取资源
POST 通常用来提交数据
```

GET 请求常见格式：

```http
GET /hello HTTP/1.1
Host: 127.0.0.1:8080

```

POST 请求常见格式：

```http
POST /echo HTTP/1.1
Host: 127.0.0.1:8080
Content-Length: 5

hello
```

HTTP 请求整体结构是：

```text
request line
headers
空行
body
```

空行之前是 headers。

空行之后是 body。

## 第 2 步：为什么不能只读一次

上一阶段我们写过：

```rust
let mut buffer = [0; 4096];
let bytes_read = stream.read(&mut buffer).unwrap();
```

这适合学习，但不够严谨。

原因是：TCP 是字节流，不是“HTTP 请求包”。

客户端发送的一个 HTTP 请求，可能被服务端分多次读到。

比如客户端发送：

```text
POST /echo HTTP/1.1
Host: 127.0.0.1:8080
Content-Length: 11

hello world
```

服务端第一次 `read` 可能只读到：

```text
POST /echo HTTP/1.1
Host: 127.0.0.1:8080
Content-Length: 11

hello
```

第二次才读到：

```text
 world
```

所以我们要循环读取，直到知道请求完整了。

## 第 3 步：如何判断请求完整

对于我们现在这个简化版 HTTP Server，可以用这个规则：

```text
1. 先读到 headers 结束，也就是找到 \r\n\r\n
2. 解析 Content-Length
3. 如果 Content-Length 是 0，请求到 headers 结束就完整了
4. 如果 Content-Length 是 N，就继续读，直到 body 有 N 个字节
```

这就是为什么 `Content-Length` 很重要。

没有它，服务端很难知道 body 到底什么时候结束。

## 第 4 步：给 Request 加 body

上一阶段的 `Request` 是：

```rust
struct Request {
    method: String,
    path: String,
    version: String,
    headers: Vec<Header>,
}
```

这一阶段加上：

```rust
body: Vec<u8>
```

代码：

```rust
// Request 表示一个 HTTP 请求。
#[derive(Debug)]
struct Request {
    // HTTP 方法，比如 GET、POST。
    method: String,

    // 请求路径，比如 /、/echo。
    path: String,

    // HTTP 版本，比如 HTTP/1.1。
    version: String,

    // 请求 headers。
    headers: Vec<Header>,

    // 请求 body。
    // 用 Vec<u8> 是因为 body 本质上是字节，不一定是文本。
    body: Vec<u8>,
}
```

## 第 5 步：查找 headers 结束位置

HTTP headers 和 body 之间用空行分隔。

在字节里，常见分隔符是：

```text
\r\n\r\n
```

也就是这 4 个字节：

```rust
b"\r\n\r\n"
```

我们写一个函数找它：

```rust
// 在 buffer 里查找 HTTP headers 结束的位置。
// 如果找到 \r\n\r\n，返回它开始的位置。
fn find_header_end(buffer: &[u8]) -> Option<usize> {
    // b"\r\n\r\n" 是字节字符串。
    let marker = b"\r\n\r\n";

    // windows(4) 会以长度 4 的窗口滑动遍历 buffer。
    // 例如 [1,2,3,4,5] 的 windows(4) 是 [1,2,3,4]、[2,3,4,5]。
    buffer
        .windows(marker.len())
        .position(|window| window == marker)
}
```

如果返回 `Some(80)`，表示：

```text
buffer[0..80]      是 request line + headers
buffer[80..84]     是 \r\n\r\n
buffer[84..]       是 body 开始
```

## 第 6 步：解析 Content-Length

我们需要从 headers 里找：

```http
Content-Length: 17
```

代码：

```rust
// 从 headers 文本里解析 Content-Length。
fn parse_content_length_from_head(head_text: &str) -> Option<usize> {
    // 按行遍历。
    for line in head_text.lines() {
        // 找到冒号，把 header 分成 name 和 value。
        let (name, value) = line.split_once(':')?;

        // HTTP header 名字大小写不敏感。
        if name.trim().eq_ignore_ascii_case("Content-Length") {
            // trim 去掉空格，再 parse 成 usize。
            return value.trim().parse::<usize>().ok();
        }
    }

    // 没有 Content-Length。
    None
}
```

这里的：

```rust
parse::<usize>()
```

意思是：把字符串解析成 `usize` 数字。

如果解析失败，`.ok()` 会把错误转换成 `None`。

## 第 7 步：循环读取完整 HTTP 请求

这是这一阶段最重要的函数。

它的目标是：

```text
从 TcpStream 里读取字节，直到一个 HTTP 请求完整
```

代码：

```rust
// 从 TCP 连接里读取一个完整 HTTP 请求。
fn read_http_request(stream: &mut TcpStream) -> Option<Vec<u8>> {
    // 用 Vec<u8> 存放所有已经读到的字节。
    let mut buffer = Vec::new();

    // 每次从 TCP 连接最多读 1024 字节。
    let mut temp = [0; 1024];

    // expected_total 表示完整请求应该有多少字节。
    // 一开始不知道，所以是 None。
    let mut expected_total: Option<usize> = None;

    loop {
        // 从 TCP 连接读取一批字节。
        let bytes_read = stream.read(&mut temp).unwrap();

        // 如果读到 0 字节，通常表示客户端关闭了连接。
        if bytes_read == 0 {
            break;
        }

        // 把这次读到的字节追加到总 buffer 里。
        buffer.extend_from_slice(&temp[..bytes_read]);	

        // 如果还不知道完整请求长度，就尝试解析 headers。
        if expected_total.is_none() {
            // 查找 headers 结束位置。
            if let Some(header_end) = find_header_end(&buffer) {
                // 只取 headers 部分转成文本。
                let head_text = String::from_utf8_lossy(&buffer[..header_end]);

                // 解析 Content-Length。
                // 如果没有 Content-Length，就当作 body 长度为 0。
                let content_length =
                    parse_content_length_from_head(&head_text).unwrap_or(0);

                // header_end 是 \r\n\r\n 开始的位置。
                // 所以 body_start 要加 4。
                let body_start = header_end + 4;

                // 完整请求长度 = body 起点 + body 长度。
                expected_total = Some(body_start + content_length);
            }
        }

        // 如果已经知道完整请求长度，就检查是否读够了。
        if let Some(total) = expected_total {
            if buffer.len() >= total {
                break;
            }
        }
    }

    // 如果一个字节都没读到，返回 None。
    if buffer.is_empty() {
        None
    } else {
        Some(buffer)
    }
}
```

这一段你刚开始可能会觉得绕。

记住它的核心就够了：

```text
不断 read
  -> 直到看到 \r\n\r\n
  -> 解析 Content-Length
  -> 继续读到 body 足够长
```

## 第 8 步：从原始字节解析 Request

现在 `parse_request` 不再接收 `&str`，而是接收 `&[u8]`。

因为我们要把 headers 当文本解析，把 body 当字节保存。

代码：

```rust
// 从完整 HTTP 请求字节中解析 Request。
fn parse_request(raw: &[u8]) -> Option<Request> {
    // 找到 headers 结束位置。
    let header_end = find_header_end(raw)?;

    // body 从 header_end + 4 开始。
    let body_start = header_end + 4;

    // headers 部分通常是 UTF-8/ASCII 文本。
    let head_text = String::from_utf8_lossy(&raw[..header_end]);

    // body 保留为字节。
    let body = raw[body_start..].to_vec();

    // 按行读取 request line 和 headers。
    let mut lines = head_text.lines();

    // 第一行是请求行。
    let request_line = lines.next()?;

    // 切分请求行。
    let mut parts = request_line.split_whitespace();

    // 解析 method、path、version。
    let method = parts.next()?.to_string();
    let path = parts.next()?.to_string();
    let version = parts.next()?.to_string();

    // 解析 headers。
    let mut headers = Vec::new();

    for line in lines {
        // 如果遇到空行就停止。
        if line.trim().is_empty() {
            break;
        }

        // 解析 header 行。
        let header = parse_header_line(line)?;

        // 加入 headers。
        headers.push(header);
    }

    // 返回 Request。
    Some(Request {
        method,
        path,
        version,
        headers,
        body,
    })
}
```

## 第 9 步：给 Request 加 body_text 方法

有时候 body 是文本，比如 curl 发的：

```bash
curl -d "hello" ...
```

为了打印方便，可以加一个方法：

```rust
impl Request {
    // 把 body 当成文本显示。
    // 如果 body 不是合法 UTF-8，就用替代字符显示。
    fn body_text(&self) -> String {
        String::from_utf8_lossy(&self.body).to_string()
    }
}
```

注意：这个方法只是为了学习和调试方便。

真实服务器不应该假设所有 body 都是文本。

## 第 10 步：支持 POST /echo

我们升级 `route`。

当请求是：

```http
POST /echo
```

就直接把客户端提交的 body 返回。

代码：

```rust
// 根据请求生成响应。
fn route(request: &Request) -> Response {	
    // 打印请求基础信息。
    println!(
        "method={}, path={}, version={}",
        request.method, request.path, request.version
    );

    // 如果有 Host header，就打印出来。
    if let Some(host) = request.header("Host") {
        println!("Host header: {}", host);
    }

    // 打印 body 长度。
    println!("body length: {}", request.body.len());

    // 根据 method 和 path 路由。
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") => {
            let body = b"<h1>Home</h1><p>Now supports POST body.</p>".to_vec();
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
            // 直接返回客户端提交的 body。
            let mut response = Response::new(200, "OK", request.body.clone());
            response.set_header("Content-Type", "text/plain; charset=utf-8");
            response
        }
        ("POST", "/submit") => {
            // 把 body 当成文本显示出来。
            let submitted = request.body_text();
            let html = format!(
                "<h1>Submitted</h1><pre>{}</pre>",
                submitted
            );
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
```

这里有一个新点：

```rust
request.body.clone()
```

`request.body` 是 `Vec<u8>`。

我们不能直接把它从 `request` 里拿走，因为 `request` 是借来的。

所以这里用 `clone()` 复制一份。

## 第 11 步：完整代码

把 `src/main.rs` 改成下面这样：

```rust
// Read 用来从 TCP 连接读取字节。
// Write 用来把响应字节写回 TCP 连接。
use std::io::{Read, Write};

// TcpListener 用来监听端口。
// TcpStream 表示一个已经建立的 TCP 连接。
use std::net::{TcpListener, TcpStream};

// Header 表示一行 HTTP header。
#[derive(Debug)]
struct Header {
    // header 名字，比如 Host、Content-Length。
    name: String,

    // header 值，比如 127.0.0.1:8080、17。
    value: String,
}

// Request 表示一个 HTTP 请求。
#[derive(Debug)]
struct Request {
    // HTTP 方法，比如 GET、POST。
    method: String,

    // 请求路径，比如 /、/echo。
    path: String,

    // HTTP 版本，比如 HTTP/1.1。
    version: String,

    // 请求 headers。
    headers: Vec<Header>,

    // 请求 body。
    body: Vec<u8>,
}

// 给 Request 添加方法。
impl Request {
    // 根据 header 名字查找 header 值。
    fn header(&self, name: &str) -> Option<&str> {
        // 遍历所有 headers。
        for header in &self.headers {
            // HTTP header 名字大小写不敏感。
            if header.name.eq_ignore_ascii_case(name) {
                return Some(header.value.as_str());
            }
        }

        // 没找到返回 None。
        None
    }

    // 把 body 当成文本显示。
    fn body_text(&self) -> String {
        // from_utf8_lossy 比 from_utf8 更宽容。
        // 遇到非法 UTF-8 字节时，它不会崩溃。
        String::from_utf8_lossy(&self.body).to_string()
    }
}

// Response 表示一个 HTTP 响应。
struct Response {
    // HTTP 状态码，比如 200、404。
    status_code: u16,

    // 状态原因短语，比如 OK、Not Found。
    reason: String,

    // 响应 headers。
    headers: Vec<Header>,

    // 响应 body。
    body: Vec<u8>,
}

// 给 Response 添加方法。
impl Response {
    // 创建一个新的 Response。
    fn new(status_code: u16, reason: &str, body: Vec<u8>) -> Response {
        // 先创建 response。
        let mut response = Response {
            status_code,
            reason: reason.to_string(),
            headers: Vec::new(),
            body,
        };

        // 自动设置 Content-Length。
        let content_length = response.body.len().to_string();
        response.set_header("Content-Length", &content_length);

        // 返回 response。
        response
    }

    // 设置 header。
    fn set_header(&mut self, name: &str, value: &str) {
        // 如果已有同名 header，就更新。
        for header in &mut self.headers {
            if header.name.eq_ignore_ascii_case(name) {
                header.value = value.to_string();
                return;
            }
        }

        // 如果没有同名 header，就新增。
        self.headers.push(Header {
            name: name.to_string(),
            value: value.to_string(),
        });
    }

    // 把 Response 编码成 HTTP 响应字节。
    fn to_http_bytes(&self) -> Vec<u8> {
        // 状态行。
        let mut head = format!("HTTP/1.1 {} {}\r\n", self.status_code, self.reason);

        // headers。
        for header in &self.headers {
            head.push_str(&format!("{}: {}\r\n", header.name, header.value));
        }

        // 空行，表示 headers 结束。
        head.push_str("\r\n");

        // 转成字节。
        let mut bytes = head.into_bytes();

        // 追加 body。
        bytes.extend_from_slice(&self.body);

        // 返回完整响应。
        bytes
    }
}

// 在字节数组里查找 headers 结束位置。
fn find_header_end(buffer: &[u8]) -> Option<usize> {
    // HTTP headers 和 body 之间的分隔符。
    let marker = b"\r\n\r\n";

    // 查找 marker 出现的位置。
    buffer
        .windows(marker.len())
        .position(|window| window == marker)
}

// 从 headers 文本中解析 Content-Length。
fn parse_content_length_from_head(head_text: &str) -> Option<usize> {
    // 遍历每一行。
    for line in head_text.lines() {
        // 如果这一行不是 header 格式，就跳过。
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };

        // 找 Content-Length。
        if name.trim().eq_ignore_ascii_case("Content-Length") {
            // 把 value 解析成 usize。
            return value.trim().parse::<usize>().ok();
        }
    }

    // 没有找到 Content-Length。
    None
}

// 解析一行 HTTP header。
fn parse_header_line(line: &str) -> Option<Header> {
    // 按第一个冒号切开。
    let (name, value) = line.split_once(':')?;

    // 去掉前后空格。
    let name = name.trim().to_string();
    let value = value.trim().to_string();

    // header 名字不能为空。
    if name.is_empty() {
        return None;
    }

    // 返回 Header。
    Some(Header { name, value })
}

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
                // headers 部分转成文本。
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

    // 没读到任何内容，返回 None。
    if buffer.is_empty() {
        None
    } else {
        Some(buffer)
    }
}

// 从完整 HTTP 请求字节解析 Request。
fn parse_request(raw: &[u8]) -> Option<Request> {
    // 找到 headers 结束位置。
    let header_end = find_header_end(raw)?;

    // body 从 header_end + 4 开始。
    let body_start = header_end + 4;

    // headers 部分按文本解析。
    let head_text = String::from_utf8_lossy(&raw[..header_end]);

    // body 部分保留为字节。
    let body = raw[body_start..].to_vec();

    // 按行读取 headers 文本。
    let mut lines = head_text.lines();

    // 第一行是请求行。
    let request_line = lines.next()?;

    // 切分请求行。
    let mut parts = request_line.split_whitespace();

    // 解析 method、path、version。
    let method = parts.next()?.to_string();
    let path = parts.next()?.to_string();
    let version = parts.next()?.to_string();

    // 保存 headers。
    let mut headers = Vec::new();

    // 解析每一行 header。
    for line in lines {
        if line.trim().is_empty() {
            break;
        }

        let header = parse_header_line(line)?;
        headers.push(header);
    }

    // 返回 Request。
    Some(Request {
        method,
        path,
        version,
        headers,
        body,
    })
}

// 根据请求生成响应。
fn route(request: &Request) -> Response {
    // 打印请求信息，方便观察。
    println!(
        "method={}, path={}, version={}",
        request.method, request.path, request.version
    );

    // 打印 Host header。
    if let Some(host) = request.header("Host") {
        println!("Host header: {}", host);
    }

    // 打印 body 长度。
    println!("body length: {}", request.body.len());

    // 根据 method 和 path 路由。
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") => {
            let body = b"<h1>Home</h1><p>Now supports POST body.</p>".to_vec();
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
            // 直接把客户端提交的 body 返回。
            let mut response = Response::new(200, "OK", request.body.clone());
            response.set_header("Content-Type", "text/plain; charset=utf-8");
            response
        }
        ("POST", "/submit") => {
            // 把 body 当成文本，放到 HTML 里。
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
    // 读取完整 HTTP 请求。
    let Some(raw_request) = read_http_request(&mut stream) else {
        return;
    };

    // 打印原始请求文本。
    println!(
        "Raw request:\n{}",
        String::from_utf8_lossy(&raw_request)
    );

    // 解析请求。
    let request = parse_request(&raw_request).unwrap();

    // 路由并生成响应。
    let response = route(&request);

    // 编码响应。
    let response_bytes = response.to_http_bytes();

    // 写回客户端。
    stream.write_all(&response_bytes).unwrap();
}

// 程序入口。
fn main() {
    // 监听本机 8080。
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    // 打印启动信息。
    println!("Server listening on http://127.0.0.1:8080");

    // 循环接受连接。
    for stream in listener.incoming() {
        // 取出 TcpStream。
        let stream = stream.unwrap();

        // 处理连接。
        handle_connection(stream);
    }
}
```

## 第 12 步：运行和测试

运行服务：

```bash
cargo run
```

测试 GET：

```bash
curl --noproxy '*' -v http://127.0.0.1:8080/
```

测试 POST echo：

```bash
curl --noproxy '*' -v -X POST -d "hello from client" http://127.0.0.1:8080/echo
```

你应该看到响应 body：

```text
hello from client
```

测试 POST submit：

```bash
curl --noproxy '*' -v -X POST -d "name=lei&project=http" http://127.0.0.1:8080/submit
```

你应该看到类似：

```html
<h1>Submitted</h1><pre>name=lei&project=http</pre>
```

## 第 13 步：观察 curl 发出的请求

运行：

```bash
curl --noproxy '*' -v -X POST -d "hello" http://127.0.0.1:8080/echo
```

你会看到 curl 发出类似请求：

```text
> POST /echo HTTP/1.1
> Host: 127.0.0.1:8080
> User-Agent: curl/8.7.1
> Accept: */*
> Content-Length: 5
> Content-Type: application/x-www-form-urlencoded
>
```

然后服务端会打印 body 长度：

```text
body length: 5
```

## 这一阶段你应该理解的问题

完成后，你应该能回答：

1. HTTP 请求的 body 在哪里开始？
2. `\r\n\r\n` 的作用是什么？
3. `Content-Length` 在请求里有什么用？
4. 为什么一次 `read` 不一定能读完整 HTTP 请求？
5. 为什么 `Request.body` 要用 `Vec<u8>`？
6. `POST /echo` 为什么要 `request.body.clone()`？

## 当前版本的局限

这个版本已经支持 request body，但仍然有局限：

- 没有处理 `Transfer-Encoding: chunked`
- 没有限制最大请求体大小
- 没有并发处理多个连接
- 没有把代码拆成模块
- 没有对错误请求返回 `400 Bad Request`
- 没有解析 query string
- 没有处理表单格式，只是把 body 当普通字节

下一阶段建议做：

- 把 HTTP 代码拆到 `src/http/` 目录
- 定义更清楚的错误类型
- 给 parser 写测试

