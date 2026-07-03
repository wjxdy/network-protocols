# 第 1 阶段：从零实现 HTTP Server

这份文档是你的第一份开发任务说明书。

目标：用 Rust 标准库写一个最小 HTTP Server，不使用任何 Web 框架。

完成后，你可以在浏览器里打开：

```text
http://127.0.0.1:8080/
```

然后看到服务端返回的内容。

## 你会学到什么

你会亲手完成：

- 创建 Rust 项目
- 监听 TCP 端口
- 接收浏览器连接
- 读取 HTTP 请求
- 打印原始请求
- 手写 HTTP 响应
- 返回 HTML
- 根据 path 做简单路由
- 返回 404
- 把代码拆成小函数
- 定义 `Request` 和 `Response`

## 第 0 步：确认 Rust 可用

在终端运行：

```bash
rustc --version
cargo --version
```

如果能看到版本号，说明 Rust 已经安装好了。

如果没有安装，可以去 Rust 官方网站安装：

```text
https://www.rust-lang.org/tools/install
```

## 第 1 步：创建 Rust 项目

在当前目录执行：

```bash
cargo init
```

执行后，目录会变成：

```text
network-protocols/
  Cargo.toml
  src/
    main.rs
  docs/
    ...
```

运行默认程序：

```bash
cargo run
```

你应该能看到：

```text
Hello, world!
```

## 第 2 步：写一个 TCP Listener

打开 `src/main.rs`，先写：

```rust
use std::net::TcpListener;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    println!("Server listening on http://127.0.0.1:8080");
}
```

运行：

```bash
cargo run
```

如果看到：

```text
Server listening on http://127.0.0.1:8080
```

说明服务端已经成功占用了 `8080` 端口。

这时程序会立刻退出，因为我们还没有写循环接受连接。

## 第 3 步：接受连接

把 `main.rs` 改成：

```rust
use std::net::TcpListener;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    println!("Server listening on http://127.0.0.1:8080");

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        println!("New connection: {:?}", stream.peer_addr());
    }
}
```

运行：

```bash
cargo run
```

另开一个终端执行：

```bash
curl http://127.0.0.1:8080/
```

你会发现 `curl` 可能会卡住。

这是正常的，因为服务端接受了连接，但没有返回任何响应。

服务端终端应该会打印类似：

```text
New connection: Ok(127.0.0.1:xxxxx)
```

## 第 4 步：读取请求

现在我们要从 `TcpStream` 里读取浏览器发来的字节。

把代码改成：

```rust
use std::io::Read;
use std::net::TcpListener;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    println!("Server listening on http://127.0.0.1:8080");

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();

        let mut buffer = [0; 1024];
        let bytes_read = stream.read(&mut buffer).unwrap();

        println!("Read {} bytes", bytes_read);

        let request_text = String::from_utf8_lossy(&buffer[..bytes_read]);
        println!("Request:\n{}", request_text);
    }
}
```

这里出现了几个重要点：

```rust
use std::io::Read;
```

导入 `Read` 之后，`TcpStream` 才能调用 `read` 方法。

```rust
let mut buffer = [0; 1024];
```

创建一个 1024 字节的缓冲区。

```rust
let bytes_read = stream.read(&mut buffer).unwrap();
```

从 TCP 连接里读取数据，写入 buffer。

```rust
&buffer[..bytes_read]
```

只取真正读到的那部分字节。

再次运行：

```bash
cargo run
```

另开终端：

```bash
curl http://127.0.0.1:8080/hello
```

你会看到类似请求：

```http
GET /hello HTTP/1.1
Host: 127.0.0.1:8080
User-Agent: curl/...
Accept: */*
```

这就是 HTTP 请求的原始样子。

## 第 5 步：返回最小 HTTP 响应

现在服务端需要写回响应，否则客户端会一直等。

HTTP 响应长这样：

```http
HTTP/1.1 200 OK
Content-Length: 13
Content-Type: text/plain

Hello, world!
```

注意：HTTP header 和 body 中间必须有一个空行。

在真实字节里，换行通常写成 `\r\n`。

把代码改成：

```rust
use std::io::{Read, Write};
use std::net::TcpListener;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    println!("Server listening on http://127.0.0.1:8080");

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();

        let mut buffer = [0; 1024];
        let bytes_read = stream.read(&mut buffer).unwrap();

        let request_text = String::from_utf8_lossy(&buffer[..bytes_read]);
        println!("Request:\n{}", request_text);

        let body = "Hello, world!";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n{}",
            body.len(),
            body
        );

        stream.write_all(response.as_bytes()).unwrap();
    }
}
```

运行：

```bash
cargo run
```

测试：

```bash
curl -v http://127.0.0.1:8080/
```

你应该能看到：

```text
Hello, world!
```

这就是你的第一个 HTTP Server。

## 第 6 步：理解 HTTP 响应格式

这一段代码很关键：

```rust
let response = format!(
    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n{}",
    body.len(),
    body
);
```

拆开看：

```http
HTTP/1.1 200 OK
```

这是状态行，表示请求成功。

```http
Content-Length: 13
```

告诉客户端 body 有多少字节。

```http
Content-Type: text/plain
```

告诉客户端 body 是纯文本。

```text
空行
```

表示 headers 结束，后面开始是 body。

```text
Hello, world!
```

这是响应体。

## 第 7 步：解析请求行

HTTP 请求的第一行叫 request line：

```http
GET /hello HTTP/1.1
```

它由三部分组成：

```text
method path version
```

我们先写一个很简单的解析。

在 `main.rs` 顶部加：

```rust
#[derive(Debug)]
struct Request {
    method: String,
    path: String,
    version: String,
}

fn parse_request(request_text: &str) -> Option<Request> {
    let request_line = request_text.lines().next()?;
    let mut parts = request_line.split_whitespace();

    let method = parts.next()?.to_string();
    let path = parts.next()?.to_string();
    let version = parts.next()?.to_string();

    Some(Request {
        method,
        path,
        version,
    })
}
```

这里用到了 `Option`。

`Option<Request>` 表示：

- 解析成功：`Some(Request)`
- 解析失败：`None`

`?` 在 `Option` 里表示：如果没有值，就直接返回 `None`。

然后在读取请求后调用：

```rust
let request = parse_request(&request_text).unwrap();
println!("Parsed request: {:?}", request);
```

完整代码暂时会变成：

```rust
use std::io::{Read, Write};
use std::net::TcpListener;

#[derive(Debug)]
struct Request {
    method: String,
    path: String,
    version: String,
}

fn parse_request(request_text: &str) -> Option<Request> {
    let request_line = request_text.lines().next()?;
    let mut parts = request_line.split_whitespace();

    let method = parts.next()?.to_string();
    let path = parts.next()?.to_string();
    let version = parts.next()?.to_string();

    Some(Request {
        method,
        path,
        version,
    })
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    println!("Server listening on http://127.0.0.1:8080");

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();

        let mut buffer = [0; 1024];
        let bytes_read = stream.read(&mut buffer).unwrap();

        let request_text = String::from_utf8_lossy(&buffer[..bytes_read]);
        println!("Request:\n{}", request_text);

        let request = parse_request(&request_text).unwrap();
        println!("Parsed request: {:?}", request);

        let body = "Hello, world!";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n{}",
            body.len(),
            body
        );

        stream.write_all(response.as_bytes()).unwrap();
    }
}
```

## 第 8 步：根据 path 返回不同内容

现在我们用 `request.path` 做一个简单路由。

把响应部分改成：

```rust
let body = match request.path.as_str() {
    "/" => "Home page",
    "/hello" => "Hello from Rust HTTP server",
    "/about" => "This server is written from zero",
    _ => "Not Found",
};
```

但这里还有一个问题：未知路径应该返回 `404 Not Found`，而不是 `200 OK`。

继续改：

```rust
let (status_line, body) = match request.path.as_str() {
    "/" => ("HTTP/1.1 200 OK", "Home page"),
    "/hello" => ("HTTP/1.1 200 OK", "Hello from Rust HTTP server"),
    "/about" => ("HTTP/1.1 200 OK", "This server is written from zero"),
    _ => ("HTTP/1.1 404 Not Found", "Not Found"),
};

let response = format!(
    "{}\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n{}",
    status_line,
    body.len(),
    body
);
```

测试：

```bash
curl -v http://127.0.0.1:8080/
curl -v http://127.0.0.1:8080/hello
curl -v http://127.0.0.1:8080/about
curl -v http://127.0.0.1:8080/missing
```

观察状态码是否正确。

## 第 9 步：返回 HTML

浏览器不仅能显示纯文本，也能显示 HTML。

把 `/` 的 body 改成：

```rust
let (status_line, content_type, body) = match request.path.as_str() {
    "/" => (
        "HTTP/1.1 200 OK",
        "text/html; charset=utf-8",
        "<h1>Home</h1><p>Hello from a tiny Rust HTTP server.</p>",
    ),
    "/hello" => (
        "HTTP/1.1 200 OK",
        "text/plain; charset=utf-8",
        "Hello from Rust HTTP server",
    ),
    _ => (
        "HTTP/1.1 404 Not Found",
        "text/plain; charset=utf-8",
        "Not Found",
    ),
};

let response = format!(
    "{}\r\nContent-Length: {}\r\nContent-Type: {}\r\n\r\n{}",
    status_line,
    body.as_bytes().len(),
    content_type,
    body
);
```

这里用 `body.as_bytes().len()` 比 `body.len()` 更明确，因为 HTTP 的 `Content-Length` 指的是字节数，不是字符数。

对于英文内容两者通常一样。对于中文，字符数和字节数不同。

## 第 10 步：把处理连接拆成函数

现在 `main` 里代码有点多。

我们把单个连接的处理拆出去：

```rust
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

#[derive(Debug)]
struct Request {
    method: String,
    path: String,
    version: String,
}

fn parse_request(request_text: &str) -> Option<Request> {
    let request_line = request_text.lines().next()?;
    let mut parts = request_line.split_whitespace();

    let method = parts.next()?.to_string();
    let path = parts.next()?.to_string();
    let version = parts.next()?.to_string();

    Some(Request {
        method,
        path,
        version,
    })
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    let bytes_read = stream.read(&mut buffer).unwrap();

    let request_text = String::from_utf8_lossy(&buffer[..bytes_read]);
    println!("Request:\n{}", request_text);

    let request = parse_request(&request_text).unwrap();
    println!("Parsed request: {:?}", request);

    let (status_line, content_type, body) = match request.path.as_str() {
        "/" => (
            "HTTP/1.1 200 OK",
            "text/html; charset=utf-8",
            "<h1>Home</h1><p>Hello from a tiny Rust HTTP server.</p>",
        ),
        "/hello" => (
            "HTTP/1.1 200 OK",
            "text/plain; charset=utf-8",
            "Hello from Rust HTTP server",
        ),
        _ => (
            "HTTP/1.1 404 Not Found",
            "text/plain; charset=utf-8",
            "Not Found",
        ),
    };

    let response = format!(
        "{}\r\nContent-Length: {}\r\nContent-Type: {}\r\n\r\n{}",
        status_line,
        body.as_bytes().len(),
        content_type,
        body
    );

    stream.write_all(response.as_bytes()).unwrap();
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    println!("Server listening on http://127.0.0.1:8080");

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(stream);
    }
}
```

现在 `main` 只负责监听和接受连接。

`handle_connection` 负责处理一个客户端连接。

## 第 11 步：当前版本的局限

这个 HTTP Server 很小，但它已经是真的 HTTP Server。

不过它还有很多限制：

- 每次只读 1024 字节
- 没有完整解析 headers
- 没有处理 request body
- 没有支持 `POST`
- 没有支持 keep-alive
- 没有并发处理多个连接
- 错误处理比较粗糙
- 所有代码还在一个文件里

这些不是失败，而是后续任务。

第一阶段最重要的是：你已经从 TCP 连接里读到了 HTTP 请求，并手动写回了 HTTP 响应。

## 第 12 步：你应该能解释这些问题

完成这一阶段后，你应该试着回答：

1. 浏览器访问 `http://127.0.0.1:8080/hello` 时，服务端读到的第一行是什么？
2. HTTP 响应为什么必须有空行？
3. `Content-Length` 是字符数还是字节数？
4. `TcpListener` 和 `TcpStream` 的区别是什么？
5. 为什么 `curl` 一开始会卡住？
6. 为什么我们暂时没有自己实现 TCP？

如果这些问题你能解释出来，说明你已经真正入门了。

## 下一步任务

完成这个最小 HTTP Server 之后，下一份开发说明书应该继续做：

- 更完整地解析 headers
- 定义 `HttpRequest`
- 定义 `HttpResponse`
- 支持 `GET` 和 `POST`
- 支持读取 request body
- 支持简单静态文件
- 添加最小测试

再往后，才进入 SSE。

