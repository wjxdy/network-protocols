# 为网络协议准备的 Rust 基础

这份文档不是完整 Rust 教程。

它只讲你写 HTTP Server、SSE、WebSocket 时马上会用到的 Rust 知识。

如果你现在完全不懂 Rust，不要怕。先掌握这些就够开始写第一个 HTTP Server。

## Rust 项目长什么样

如果你是在一个全新的目录外面创建项目，可以用：

```bash
cargo new mini-protocols
cd mini-protocols
```

如果你已经在当前项目目录里，也就是现在这个 `network-protocols` 目录里，可以用：

```bash
cargo init
```

两种方式都会得到类似结构：

```text
network-protocols/
  Cargo.toml
  src/
    main.rs
```

`Cargo.toml` 是项目配置文件。

`src/main.rs` 是程序入口。

运行：

```bash
cargo run
```

检查代码能不能编译：

```bash
cargo check
```

## main 函数

Rust 程序从 `main` 函数开始：

```rust
fn main() {
    println!("Hello, world!");
}
```

`fn` 表示定义函数。

`main` 是入口函数。

`println!` 是打印到终端的宏。它后面有一个 `!`，说明它不是普通函数，而是宏。

## let 和 mut

Rust 默认变量不能修改：

```rust
let name = "Alice";
```

如果你想修改变量，需要加 `mut`：

```rust
let mut count = 0;
count = count + 1;
```

网络编程里经常需要可变变量，比如 buffer：

```rust
let mut buffer = [0; 1024];
```

这行代码表示：创建一个长度为 1024 的数组，里面每个元素都是 0。

## 字符串：String 和 &str

Rust 里常见两种字符串：

```rust
let a: &str = "hello";
let b: String = String::from("hello");
```

简单理解：

- `&str` 是借来的字符串片段
- `String` 是自己拥有的、可以增长的字符串

HTTP 请求本质上是字节，但它通常可以按文本查看：

```rust
let request_text = String::from_utf8_lossy(&buffer);
```

这表示：把字节转换成可以看的字符串。

## 字节：u8、Vec<u8>、&[u8]

网络里传输的不是 Rust 字符串，而是字节。

一个字节是 `u8`：

```rust
let byte: u8 = 65;
```

一串字节可以用 `Vec<u8>`：

```rust
let data: Vec<u8> = vec![72, 101, 108, 108, 111];
```

也可以用字节字符串：

```rust
let data = b"Hello";
```

`b"Hello"` 的类型是字节数组，不是普通字符串。

写 HTTP 响应时，我们经常会写：

```rust
stream.write_all(response.as_bytes())?;
```

`as_bytes()` 会把字符串变成字节切片 `&[u8]`。

## Result：处理可能失败的操作

网络操作经常失败。

比如：

- 端口被占用
- 客户端断开
- 读取失败
- 写入失败

Rust 用 `Result` 表示可能成功，也可能失败：

```rust
Result<T, E>
```

意思是：

- 成功时得到 `T`
- 失败时得到 `E`

比如 `TcpListener::bind` 会返回一个 `Result`：

```rust
let listener = std::net::TcpListener::bind("127.0.0.1:8080").unwrap();
```

这里的 `unwrap()` 表示：如果成功，就取出结果；如果失败，程序直接崩溃并打印错误。

初学阶段可以先用 `unwrap()`。等你熟悉后，再改成更好的错误处理。

## ? 运算符

如果一个函数返回 `Result`，可以用 `?` 快速传播错误：

```rust
use std::io;

fn run() -> io::Result<()> {
    let listener = std::net::TcpListener::bind("127.0.0.1:8080")?;
    Ok(())
}
```

`?` 的意思是：

- 如果成功，取出里面的值
- 如果失败，直接把错误返回给调用者

`Ok(())` 表示成功结束，但没有额外返回值。

## use：导入名字

如果每次都写完整路径会很长：

```rust
std::net::TcpListener
std::io::Read
std::io::Write
```

可以用 `use` 导入：

```rust
use std::net::TcpListener;
use std::io::{Read, Write};
```

之后就可以直接写：

```rust
let listener = TcpListener::bind("127.0.0.1:8080")?;
```

## trait：Read 和 Write

Rust 里很多能力通过 trait 表达。

你现在可以先把 trait 理解成“某种能力”。

`Read` 表示这个东西可以读取字节。

`Write` 表示这个东西可以写入字节。

`TcpStream` 同时实现了 `Read` 和 `Write`，所以它既能读请求，也能写响应：

```rust
stream.read(&mut buffer)?;
stream.write_all(response.as_bytes())?;
```

## TcpListener 和 TcpStream

`TcpListener` 用来监听端口：

```rust
let listener = TcpListener::bind("127.0.0.1:8080")?;
```

`TcpStream` 表示一个 TCP 连接：

```rust
for stream in listener.incoming() {
    let mut stream = stream?;
}
```

每当一个客户端连接进来，`listener.incoming()` 就会给你一个 `TcpStream`。

你可以从里面读请求，也可以往里面写响应。

## match：根据不同情况处理

`match` 很像其他语言里的 `switch`，但更强大。

例子：

```rust
match path {
    "/" => "Home",
    "/hello" => "Hello",
    _ => "Not Found",
}
```

`_` 表示其他所有情况。

做 HTTP 路由时，`match` 很有用。

## struct：定义自己的数据结构

HTTP 请求可以定义成结构体：

```rust
struct Request {
    method: String,
    path: String,
    version: String,
}
```

创建：

```rust
let request = Request {
    method: String::from("GET"),
    path: String::from("/"),
    version: String::from("HTTP/1.1"),
};
```

访问字段：

```rust
println!("{}", request.path);
```

## impl：给结构体添加方法

```rust
struct Response {
    status_code: u16,
    body: String,
}

impl Response {
    fn to_http_string(&self) -> String {
        format!(
            "HTTP/1.1 {} OK\r\nContent-Length: {}\r\n\r\n{}",
            self.status_code,
            self.body.len(),
            self.body
        )
    }
}
```

`&self` 表示这个方法会借用当前对象。

`format!` 和 `println!` 类似，但它返回一个字符串，而不是打印出来。

## 模块：mod

刚开始所有代码都写在 `main.rs` 里没问题。

后面代码多了，可以拆文件。

比如：

```text
src/
  main.rs
  http.rs
```

在 `main.rs` 里写：

```rust
mod http;
```

这样就能使用 `http.rs` 里的内容。

再后面可以拆成目录：

```text
src/
  http/
    mod.rs
    request.rs
    response.rs
```

但第一阶段先不要急着拆。

## 写网络协议时最重要的 Rust 心法

### 1. 先分清字符串和字节

HTTP 看起来是文本，但网络里传的是字节。

你要经常在 `String` 和 `&[u8]` 之间转换。

### 2. 先允许 unwrap，后面再优化错误处理

初学阶段不要被错误处理拖住。

第一版可以写：

```rust
let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
```

理解之后，再改成：

```rust
fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    Ok(())
}
```

### 3. 小步运行

不要一次写 100 行再运行。

推荐节奏：

```text
写 5 到 15 行
cargo check
cargo run
curl 测一下
再继续
```
