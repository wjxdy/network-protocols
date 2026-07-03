# 第 1 阶段：从零实现 HTTP Server

这份文档是你的第一份开发任务说明书。

目标：用 Rust 标准库写一个最小 HTTP Server，不使用任何 Web 框架。

完成后，你可以在浏览器里打开：

```text
http://127.0.0.1:8080/
```

然后看到服务端返回的内容。

## 如何阅读代码注释

这份文档里的 Rust 代码会尽量写注释。

注释长这样：

```rust
// 这是注释，不会被 Rust 执行，只是写给人看的说明
```

前面的步骤会解释关键代码。到最后的完整版本时，我会把代码写成接近逐行注释的形式。你刚开始可以照着带注释版本敲，等熟悉之后，再自己尝试删掉注释，只保留代码。

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
// 从 Rust 标准库里导入 TcpListener。
// TcpListener 的作用是监听一个 TCP 地址和端口，等待客户端连接。
use std::net::TcpListener;

// main 是 Rust 程序的入口函数，程序会从这里开始执行。
fn main() {
    // 在本机 127.0.0.1 的 8080 端口上启动监听。
    // bind 可能失败，比如端口被占用，所以它返回 Result。
    // unwrap 表示：成功就取出 TcpListener，失败就让程序直接崩溃并打印错误。
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    // 在终端打印提示，告诉我们服务端已经启动。
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
// 导入 TCP 监听器。
use std::net::TcpListener;

// 程序入口。
fn main() {
    // 监听本机 8080 端口。
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    // 打印服务端地址。
    println!("Server listening on http://127.0.0.1:8080");

    // incoming 会不断等待新的 TCP 连接。
    // 每来一个客户端连接，循环体就执行一次。
    for stream in listener.incoming() {
        // stream 是 Result<TcpStream, Error>。
        // unwrap 后得到真正的 TcpStream。
        let stream = stream.unwrap();

        // 打印客户端地址。
        // peer_addr 也可能失败，所以这里打印出来的是一个 Result。
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
// 导入 Read trait。
// 有了它，TcpStream 才能调用 read 方法读取字节。
use std::io::Read;

// 导入 TCP 监听器。
use std::net::TcpListener;

// 程序入口。
fn main() {
    // 监听本机 8080 端口。
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    // 打印启动信息。
    println!("Server listening on http://127.0.0.1:8080");

    // 不断等待客户端连接。
    for stream in listener.incoming() {
        // 取出 TcpStream。
        // 这里必须写 mut，因为下面 read 会修改 stream 的内部读取状态。
        let mut stream = stream.unwrap();

        // 创建 1024 字节的缓冲区，用来暂时存放客户端发来的数据。
        let mut buffer = [0; 1024];

        // 从 TCP 连接读取数据到 buffer 里。
        // 返回值 bytes_read 表示这次实际读到了多少字节。
        let bytes_read = stream.read(&mut buffer).unwrap();

        // 打印读取到的字节数。
        println!("Read {} bytes", bytes_read);

        // buffer 里可能不是 1024 字节都有效。
        // 所以只取前 bytes_read 个字节，并把它们按 UTF-8 文本显示出来。
        let request_text = String::from_utf8_lossy(&buffer[..bytes_read]);

        // 打印原始 HTTP 请求。
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
// 同时导入 Read 和 Write。
// Read 用来读请求，Write 用来写响应。
use std::io::{Read, Write};

// 导入 TCP 监听器。
use std::net::TcpListener;

// 程序入口。
fn main() {
    // 监听本机 8080 端口。
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    // 打印启动信息。
    println!("Server listening on http://127.0.0.1:8080");

    // 不断接受客户端连接。
    for stream in listener.incoming() {
        // 取出 TcpStream，并声明为可变，因为要读写它。
        let mut stream = stream.unwrap();

        // 创建读取缓冲区。
        let mut buffer = [0; 1024];

        // 读取客户端请求。
        let bytes_read = stream.read(&mut buffer).unwrap();

        // 把读到的字节转换成方便查看的文本。
        let request_text = String::from_utf8_lossy(&buffer[..bytes_read]);

        // 打印请求内容。
        println!("Request:\n{}", request_text);

        // 定义响应 body，也就是真正要返回给客户端的内容。
        let body = "Hello, world!";

        // 构造完整 HTTP 响应。
        // 注意 headers 和 body 之间有一个空行：\r\n\r\n。
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n{}",
            body.len(),
            body
        );

        // 把响应字符串转换成字节，然后写回 TCP 连接。
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
// 自动实现 Debug，这样 Request 可以用 {:?} 打印出来。
#[derive(Debug)]
// 定义一个结构体，用来保存 HTTP 请求的第一行信息。
struct Request {
    // HTTP 方法，比如 GET、POST。
    method: String,

    // 请求路径，比如 /、/hello。
    path: String,

    // HTTP 版本，比如 HTTP/1.1。
    version: String,
}

// 解析 HTTP 请求文本，成功时返回 Some(Request)，失败时返回 None。
fn parse_request(request_text: &str) -> Option<Request> {
    // 取请求文本的第一行。
    // 如果没有第一行，? 会让整个函数直接返回 None。
    let request_line = request_text.lines().next()?;

    // 按空白字符切分第一行。
    // "GET /hello HTTP/1.1" 会被切成三段。
    let mut parts = request_line.split_whitespace();

    // 取第一段作为 method，并转换成 String。
    let method = parts.next()?.to_string();

    // 取第二段作为 path。
    let path = parts.next()?.to_string();

    // 取第三段作为 version。
    let version = parts.next()?.to_string();

    // 把解析出来的字段放进 Request 结构体里。
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
// 同时导入 Read 和 Write。
use std::io::{Read, Write};

// 导入 TCP 监听器。
use std::net::TcpListener;

// 让 Request 可以用 {:?} 打印，方便调试。
#[derive(Debug)]
// 保存 HTTP 请求第一行解析后的结果。
struct Request {
    // 请求方法，例如 GET。
    method: String,

    // 请求路径，例如 /hello。
    path: String,

    // HTTP 版本，例如 HTTP/1.1。
    version: String,
}

// 把原始 HTTP 请求文本解析成 Request。
fn parse_request(request_text: &str) -> Option<Request> {
    // 获取第一行，比如 "GET /hello HTTP/1.1"。
    let request_line = request_text.lines().next()?;

    // 按空白分割第一行。
    let mut parts = request_line.split_whitespace();

    // 第一段是 HTTP 方法。
    let method = parts.next()?.to_string();

    // 第二段是请求路径。
    let path = parts.next()?.to_string();

    // 第三段是 HTTP 版本。
    let version = parts.next()?.to_string();

    // 组装成 Request 并返回。
    Some(Request {
        method,
        path,
        version,
    })
}

// 程序入口。
fn main() {
    // 监听本机 8080 端口。
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    // 打印启动信息。
    println!("Server listening on http://127.0.0.1:8080");

    // 循环接受客户端连接。
    for stream in listener.incoming() {
        // 取出连接，并声明为可变。
        let mut stream = stream.unwrap();

        // 准备 1024 字节缓冲区。
        let mut buffer = [0; 1024];

        // 读取客户端发来的 HTTP 请求。
        let bytes_read = stream.read(&mut buffer).unwrap();

        // 把实际读到的字节转成文本。
        let request_text = String::from_utf8_lossy(&buffer[..bytes_read]);

        // 打印原始请求。
        println!("Request:\n{}", request_text);

        // 解析请求行。
        let request = parse_request(&request_text).unwrap();

        // 打印解析后的结构体。
        println!("Parsed request: {:?}", request);

        // 准备响应 body。
        let body = "Hello, world!";

        // 拼出 HTTP 响应。
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: text/plain\r\n\r\n{}",
            body.len(),
            body
        );

        // 写回响应。
        stream.write_all(response.as_bytes()).unwrap();
    }
}
```

## 第 8 步：根据 path 返回不同内容

现在我们用 `request.path` 做一个简单路由。

把响应部分改成：

```rust
// 根据请求路径决定返回什么 body。
let body = match request.path.as_str() {
    // 如果访问 /，返回首页文本。
    "/" => "Home page",

    // 如果访问 /hello，返回 hello 文本。
    "/hello" => "Hello from Rust HTTP server",

    // 如果访问 /about，返回 about 文本。
    "/about" => "This server is written from zero",

    // _ 表示其他所有路径。
    _ => "Not Found",
};
```

但这里还有一个问题：未知路径应该返回 `404 Not Found`，而不是 `200 OK`。

继续改：

```rust
// 根据请求路径，同时决定状态行和响应 body。
let (status_line, body) = match request.path.as_str() {
    // 已知路径返回 200 OK。
    "/" => ("HTTP/1.1 200 OK", "Home page"),
    "/hello" => ("HTTP/1.1 200 OK", "Hello from Rust HTTP server"),
    "/about" => ("HTTP/1.1 200 OK", "This server is written from zero"),

    // 未知路径返回 404 Not Found。
    _ => ("HTTP/1.1 404 Not Found", "Not Found"),
};

// 把状态行、headers、body 拼成完整 HTTP 响应。
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
// 根据请求路径决定 status、Content-Type 和 body。
let (status_line, content_type, body) = match request.path.as_str() {
    // 首页返回 HTML。
    "/" => (
        "HTTP/1.1 200 OK",
        "text/html; charset=utf-8",
        "<h1>Home</h1><p>Hello from a tiny Rust HTTP server.</p>",
    ),

    // /hello 返回纯文本。
    "/hello" => (
        "HTTP/1.1 200 OK",
        "text/plain; charset=utf-8",
        "Hello from Rust HTTP server",
    ),

    // 其他路径返回 404。
    _ => (
        "HTTP/1.1 404 Not Found",
        "text/plain; charset=utf-8",
        "Not Found",
    ),
};

// 构造 HTTP 响应。
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
// 从标准库导入 Read 和 Write。
// Read 让 TcpStream 可以读取客户端发来的字节。
// Write 让 TcpStream 可以把响应字节写回客户端。
use std::io::{Read, Write};

// 从标准库导入两个 TCP 类型。
// TcpListener 用来监听端口。
// TcpStream 表示一个已经建立好的 TCP 连接。
use std::net::{TcpListener, TcpStream};

// derive(Debug) 的意思是：让这个结构体可以用 {:?} 打印出来。
// 这对初学阶段调试非常有用。
#[derive(Debug)]
// 定义一个 Request 结构体，用来保存 HTTP 请求第一行的信息。
struct Request {
    // method 表示 HTTP 方法，比如 GET、POST。
    // 这里先用 String 保存，简单直接。
    method: String,

    // path 表示请求路径，比如 /、/hello、/about。
    path: String,

    // version 表示 HTTP 版本，比如 HTTP/1.1。
    version: String,
}

// 定义一个函数，把原始 HTTP 请求文本解析成 Request。
// 参数 request_text: &str 表示：借用一段字符串，不拿走它的所有权。
// 返回 Option<Request> 表示：可能解析成功，也可能解析失败。
fn parse_request(request_text: &str) -> Option<Request> {
    // request_text.lines() 会按行遍历请求文本。
    // next() 取第一行，也就是 HTTP request line。
    // 如果没有第一行，? 会让函数直接返回 None。
    let request_line = request_text.lines().next()?;

    // split_whitespace 会按空白字符切分字符串。
    // 例如 "GET /hello HTTP/1.1" 会切成 GET、/hello、HTTP/1.1。
    let mut parts = request_line.split_whitespace();

    // 取第一段作为 method。
    // to_string() 把 &str 转成拥有所有权的 String。
    let method = parts.next()?.to_string();

    // 取第二段作为 path。
    let path = parts.next()?.to_string();

    // 取第三段作为 version。
    let version = parts.next()?.to_string();

    // 把解析结果放入 Request 结构体，并用 Some 包起来表示成功。
    Some(Request {
        method,
        path,
        version,
    })
}

// 处理一个客户端连接。
// 参数 mut stream: TcpStream 表示接收一个 TCP 连接，并且这个连接是可变的。
// 它需要可变，是因为 read/write 会改变连接内部的读取和写入状态。
fn handle_connection(mut stream: TcpStream) {
    // 创建一个 1024 字节的数组，作为临时读取缓冲区。
    // [0; 1024] 表示数组长度是 1024，每个位置的初始值都是 0。
    let mut buffer = [0; 1024];

    // 从 TCP 连接读取数据到 buffer。
    // bytes_read 表示实际读到了多少字节。
    // unwrap 表示：如果读取失败，程序直接报错退出。
    let bytes_read = stream.read(&mut buffer).unwrap();

    // buffer 可能没有被填满。
    // &buffer[..bytes_read] 表示只取实际读到的那部分字节。
    // String::from_utf8_lossy 会把字节转换成可打印的文本。
    let request_text = String::from_utf8_lossy(&buffer[..bytes_read]);

    // 打印原始 HTTP 请求，方便你观察浏览器或 curl 到底发了什么。
    println!("Request:\n{}", request_text);

    // 调用 parse_request 解析请求第一行。
    // unwrap 表示这里先假设请求格式一定正确。
    let request = parse_request(&request_text).unwrap();

    // 打印解析后的 Request。
    println!("Parsed request: {:?}", request);

    // 根据请求路径决定返回什么内容。
    // match 很像其他语言里的 switch。
    // 这里同时返回三个值：状态行、内容类型、响应体。
    let (status_line, content_type, body) = match request.path.as_str() {
        // 如果访问根路径 /，返回 HTML。
        "/" => (
            "HTTP/1.1 200 OK",
            "text/html; charset=utf-8",
            "<h1>Home</h1><p>Hello from a tiny Rust HTTP server.</p>",
        ),

        // 如果访问 /hello，返回纯文本。
        "/hello" => (
            "HTTP/1.1 200 OK",
            "text/plain; charset=utf-8",
            "Hello from Rust HTTP server",
        ),

        // _ 表示其他所有路径。
        // 对未知路径返回 404。
        _ => (
            "HTTP/1.1 404 Not Found",
            "text/plain; charset=utf-8",
            "Not Found",
        ),
    };

    // 使用 format! 拼出完整 HTTP 响应字符串。
    // HTTP 响应由三部分组成：
    // 1. 状态行
    // 2. headers
    // 3. 空行后面的 body
    //
    // \r\n 是 HTTP 常用换行。
    // \r\n\r\n 表示 headers 结束，后面开始是 body。
    let response = format!(
        "{}\r\nContent-Length: {}\r\nContent-Type: {}\r\n\r\n{}",
        status_line,

        // Content-Length 要写 body 的字节数。
        // 用 as_bytes().len() 比 body.len() 更明确。
        body.as_bytes().len(),

        // Content-Type 告诉浏览器如何理解 body。
        content_type,

        // body 是真正返回给客户端看的内容。
        body
    );

    // 把响应字符串转换成字节，写回 TCP 连接。
    // HTTP 本质上就是通过 TCP 发送一串符合格式的字节。
    stream.write_all(response.as_bytes()).unwrap();
}

// main 是程序入口。
fn main() {
    // 在本机 127.0.0.1 的 8080 端口启动 TCP 监听。
    // 127.0.0.1 表示只允许本机访问。
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    // 打印服务启动信息。
    println!("Server listening on http://127.0.0.1:8080");

    // listener.incoming() 会不断等待新的客户端连接。
    // 每有一个连接进来，for 循环就执行一次。
    for stream in listener.incoming() {
        // stream 是 Result<TcpStream, Error>。
        // unwrap 后得到真正的 TcpStream。
        let stream = stream.unwrap();

        // 把这个连接交给 handle_connection 处理。
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
