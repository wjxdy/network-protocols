# 第 4 阶段：把 HTTP 代码拆成模块，并给解析器写测试

前面几版代码都写在 `src/main.rs` 里。

这对初学很友好，因为所有东西都在一个文件里，方便你从上往下看。

但现在代码已经开始变长了。后面还要做：

- SSE
- WebSocket
- HTTP Client
- 教学版 TCP

如果所有代码都堆在 `main.rs`，后面会越来越难维护。

这一阶段我们做两件事：

```text
1. 把 HTTP 相关代码拆到 src/http/ 目录
2. 给 HTTP parser 写最小测试
```

## 这一阶段的目标

完成后，项目结构会变成：

```text
src/
  main.rs
  http/
    mod.rs
    header.rs
    request.rs
    response.rs
    parser.rs
    server.rs
```

每个文件负责一件事：

```text
header.rs    Header 结构体
request.rs   Request 结构体和方法
response.rs  Response 结构体和响应编码
parser.rs    解析 HTTP 请求
server.rs    读取 TCP 连接、处理连接
mod.rs       把 http 模块组合起来
main.rs      程序入口，只负责启动服务
```

## 为什么现在要拆模块

拆模块不是为了炫技，而是为了降低理解难度。

现在 `main.rs` 里混着很多职责：

```text
监听端口
读取 TCP 数据
解析 HTTP 请求
构造 HTTP 响应
路由
测试 parser
```

拆开以后，每个文件更像一个小抽屉：

```text
想看请求结构 -> request.rs
想看响应结构 -> response.rs
想看解析逻辑 -> parser.rs
想看 TCP 读取 -> server.rs
```

这对后面做 SSE 和 WebSocket 很重要。

## 第 1 步：创建目录和文件

在项目根目录执行：

```bash
mkdir -p src/http
touch src/http/mod.rs
touch src/http/header.rs
touch src/http/request.rs
touch src/http/response.rs
touch src/http/parser.rs
touch src/http/server.rs
```

执行后，目录应该类似：

```text
src/
  main.rs
  http/
    mod.rs
    header.rs
    request.rs
    response.rs
    parser.rs
    server.rs
```

## 第 2 步：理解 `mod` 和 `pub`

Rust 里，一个文件默认不是自动可见的。

如果你创建了：

```text
src/http/header.rs
```

你还需要在：

```text
src/http/mod.rs
```

里声明：

```rust
pub mod header;
```

这句话的意思是：

```text
http 模块下面有一个 header 子模块，并且对外公开
```

`pub` 是 public 的意思，表示公开。

如果没有 `pub`，外面的模块就不能访问它。

## 第 3 步：写 `src/http/mod.rs`

`mod.rs` 是 `http` 模块的入口文件。

写入：

```rust
// 声明 http 模块下面有哪些子模块。
// pub 表示这些模块可以被 http 模块外部访问。
pub mod header;
pub mod parser;
pub mod request;
pub mod response;
pub mod server;

// 重新导出常用类型。
// 这样外部可以写 http::Request，而不是 http::request::Request。
pub use request::Request;
pub use response::Response;
```

这一段里：

```rust
pub use request::Request;
```

可以理解成给外部提供一个更短的访问路径。

## 第 4 步：写 `src/http/header.rs`

把 `Header` 放进 `header.rs`：

```rust
// Header 表示一行 HTTP header。
//
// 例如：
// Host: 127.0.0.1:8080
//
// name  = Host
// value = 127.0.0.1:8080
#[derive(Debug, Clone)]
pub struct Header {
    // pub 表示外部模块可以读取这个字段。
    pub name: String,

    // header 的值。
    pub value: String,
}

impl Header {
    // 创建一个 Header。
    pub fn new(name: &str, value: &str) -> Header {
        Header {
            name: name.to_string(),
            value: value.to_string(),
        }
    }
}
```

这里加了：

```rust
Clone
```

意思是 `Header` 可以复制一份。

后面如果需要复制 headers，会方便一些。

## 第 5 步：写 `src/http/request.rs`

把 `Request` 放进 `request.rs`：

```rust
// 从同一个 http 模块下的 header 子模块导入 Header。
use super::header::Header;

// Request 表示一个 HTTP 请求。
#[derive(Debug)]
pub struct Request {
    // HTTP 方法，比如 GET、POST。
    pub method: String,

    // 请求路径，比如 /、/echo。
    pub path: String,

    // HTTP 版本，比如 HTTP/1.1。
    pub version: String,

    // 请求 headers。
    pub headers: Vec<Header>,

    // 请求 body。
    pub body: Vec<u8>,
}

impl Request {
    // 根据 header 名字查找 header 值。
    pub fn header(&self, name: &str) -> Option<&str> {
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
    pub fn body_text(&self) -> String {
        String::from_utf8_lossy(&self.body).to_string()
    }
}
```

这里的：

```rust
use super::header::Header;
```

意思是：

```text
从父模块 http 下面的 header 模块里，导入 Header
```

因为 `request.rs` 和 `header.rs` 都在 `src/http/` 目录下，所以用 `super`。

## 第 6 步：写 `src/http/response.rs`

把 `Response` 放进 `response.rs`：

```rust
// 导入 Header。
use super::header::Header;

// Response 表示一个 HTTP 响应。
pub struct Response {
    // HTTP 状态码，比如 200、404。
    pub status_code: u16,

    // 状态原因短语，比如 OK、Not Found。
    pub reason: String,

    // 响应 headers。
    pub headers: Vec<Header>,

    // 响应 body。
    pub body: Vec<u8>,
}

impl Response {
    // 创建一个新的 Response。
    pub fn new(status_code: u16, reason: &str, body: Vec<u8>) -> Response {
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

        response
    }

    // 设置 header。
    pub fn set_header(&mut self, name: &str, value: &str) {
        // 如果已有同名 header，就更新。
        for header in &mut self.headers {
            if header.name.eq_ignore_ascii_case(name) {
                header.value = value.to_string();
                return;
            }
        }

        // 如果没有同名 header，就新增。
        self.headers.push(Header::new(name, value));
    }

    // 把 Response 编码成 HTTP 响应字节。
    pub fn to_http_bytes(&self) -> Vec<u8> {
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

        bytes
    }
}
```

## 第 7 步：写 `src/http/parser.rs`

把解析相关函数放进 `parser.rs`：

```rust
// 导入 Header 和 Request。
use super::header::Header;
use super::request::Request;

// 在字节数组里查找 headers 结束位置。
pub fn find_header_end(buffer: &[u8]) -> Option<usize> {
    // HTTP headers 和 body 之间的分隔符。
    let marker = b"\r\n\r\n";

    // 查找 marker 出现的位置。
    buffer
        .windows(marker.len())
        .position(|window| window == marker)
}

// 从 headers 文本中解析 Content-Length。
pub fn parse_content_length_from_head(head_text: &str) -> Option<usize> {
    // 遍历每一行。
    for line in head_text.lines() {
        // 请求行不是 header，没有冒号，所以这里要跳过。
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };

        // 找 Content-Length。
        if name.trim().eq_ignore_ascii_case("Content-Length") {
            return value.trim().parse::<usize>().ok();
        }
    }

    None
}

// 解析一行 HTTP header。
pub fn parse_header_line(line: &str) -> Option<Header> {
    // 按第一个冒号切开。
    let (name, value) = line.split_once(':')?;

    // 去掉前后空格。
    let name = name.trim().to_string();
    let value = value.trim().to_string();

    // header 名字不能为空。
    if name.is_empty() {
        return None;
    }

    Some(Header { name, value })
}

// 从完整 HTTP 请求字节解析 Request。
pub fn parse_request(raw: &[u8]) -> Option<Request> {
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
        let header = parse_header_line(line)?;
        headers.push(header);
    }

    Some(Request {
        method,
        path,
        version,
        headers,
        body,
    })
}

// cfg(test) 表示：下面这个模块只在 cargo test 时编译。
#[cfg(test)]
mod tests {
    // use super::* 表示导入当前文件里的所有函数。
    use super::*;

    #[test]
    fn finds_header_end() {
        let raw = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        assert_eq!(find_header_end(raw), Some(31));
    }

    #[test]
    fn parses_content_length() {
        let head = "POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: 5";
        assert_eq!(parse_content_length_from_head(head), Some(5));
    }

    #[test]
    fn parses_header_line() {
        let header = parse_header_line("Host: 127.0.0.1:8080").unwrap();
        assert_eq!(header.name, "Host");
        assert_eq!(header.value, "127.0.0.1:8080");
    }

    #[test]
    fn parses_post_request_with_body() {
        let raw = b"POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: 5\r\n\r\nhello";
        let request = parse_request(raw).unwrap();

        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/echo");
        assert_eq!(request.version, "HTTP/1.1");
        assert_eq!(request.header("Host"), Some("localhost"));
        assert_eq!(request.body, b"hello");
    }
}
```

这里第一次写测试。

测试函数长这样：

```rust
#[test]
fn parses_header_line() {
    ...
}
```

运行：

```bash
cargo test
```

Rust 会自动找到所有带 `#[test]` 的函数并执行。

## 第 8 步：写 `src/http/server.rs`

把 TCP 读取、路由、连接处理放进 `server.rs`：

```rust
// Read 用来从 TCP 连接读取字节。
// Write 用来把响应字节写回 TCP 连接。
use std::io::{Read, Write};

// TcpListener 用来监听端口。
// TcpStream 表示一个已经建立的 TCP 连接。
use std::net::{TcpListener, TcpStream};

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
                // 不能取 temp，也不能取 buffer[..bytes_read]。
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

// 根据请求生成响应。
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
            let body = b"<h1>Home</h1><p>Code is now split into modules.</p>".to_vec();
            let mut response = Response::new(200, "OK", body);
            response.set_header("Content-Type", "text/html; charset=utf-8");
            response
        }
        ("GET", "/hello") => {
            let body = b"Hello from modular Rust HTTP server".to_vec();
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
    let response = route(&request);
    let response_bytes = response.to_http_bytes();

    stream.write_all(&response_bytes).unwrap();
}

// 启动 HTTP Server。
pub fn serve(addr: &str) {
    // 监听传入的地址。
    let listener = TcpListener::bind(addr).unwrap();

    println!("Server listening on http://{}", addr);

    // 循环接受连接。
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(stream);
    }
}
```

注意这个函数：

```rust
pub fn serve(addr: &str)
```

它是公开的，所以 `main.rs` 可以调用它。

## 第 9 步：简化 `src/main.rs`

现在 `main.rs` 只需要负责启动服务。

写成：

```rust
// 声明 http 模块。
mod http;

// 程序入口。
fn main() {
    // 启动 HTTP Server。
    http::server::serve("127.0.0.1:8080");
}
```

这就是拆模块后的好处：

```text
main.rs 不再关心 HTTP 解析细节
main.rs 只负责启动程序
```

## 第 10 步：运行测试

先运行测试：

```bash
cargo test
```

你应该看到类似：

```text
running 4 tests
test http::parser::tests::finds_header_end ... ok
test http::parser::tests::parses_content_length ... ok
test http::parser::tests::parses_header_line ... ok
test http::parser::tests::parses_post_request_with_body ... ok
```

如果测试失败，不要急着改一堆。

先看失败信息，确认是哪一个断言失败。

## 第 11 步：运行服务

运行：

```bash
cargo run
```

测试 GET：

```bash
curl --noproxy '*' -v http://127.0.0.1:8080/
```

测试 POST：

```bash
curl --noproxy '*' -v -X POST -d "hello module" http://127.0.0.1:8080/echo
```

你应该看到：

```text
hello module
```

## 第 12 步：你应该理解的问题

完成后，你应该能回答：

1. `mod http;` 的作用是什么？
2. `src/http/mod.rs` 的作用是什么？
3. `pub mod parser;` 和 `mod parser;` 有什么区别？
4. 为什么 `Request` 的字段要加 `pub`？
5. `use super::header::Header;` 里的 `super` 是什么意思？
6. `cargo test` 会运行哪些函数？
7. 为什么 parser 的测试应该放在 `parser.rs` 里？

## 当前版本的局限

这个版本已经比前面更像一个真正项目了，但还有局限：

- 错误处理还是大量 `unwrap`
- 读请求没有最大大小限制
- 路由还写死在 `server.rs`
- 没有并发处理连接
- 没有支持静态文件
- 没有支持 SSE

下一阶段建议做：

```text
SSE: Server-Sent Events
```

因为 SSE 正好建立在 HTTP 长连接之上，很适合接着现在的 HTTP Server 往前走。
