use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

#[derive(Debug)]
struct Request {
    method: String,

    path: String,

    version: String,

    headers: Vec<Header>,
}

impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        for header in &self.headers {
            if header.name.eq_ignore_ascii_case(name) {
                return Some(header.value.as_str());
            }
        }

        None
    }
}


struct Response {
    status_code: u16,
    reason: String,
    headers: Vec<Header>,
    body: Vec<u8>,
}



#[derive(Debug)]
struct Header {
    name: String,
    value: String,
}

fn parse_header_line(line: &str) -> Option<Header> {
    let (name, value) = line.split_once(":")?;

    let name = name.trim().to_string();
    let value = value.trim().to_string();

    if name.is_empty() {
        return None;
    }

    Some(Header { name, value })
}

fn parse_request(request_text: &str) -> Option<Request> {
    let mut lines = request_text.lines();

    let request_line = lines.next()?;

    let mut parts = request_line.split_whitespace();

    let method = parts.next()?.to_string();

    let path = parts.next()?.to_string();

    let version = parts.next()?.to_string();

    let mut headers = Vec::new();

    for line in lines {
        if line.trim().is_empty() {
            break;
        }

        let header = parse_header_line(line)?;

        headers.push(header);
    }

    Some(Request {
        method,
        path,
        version,
        headers,
    })
}

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 1024];

    let bytes_read = stream.read(&mut buffer).unwrap();

    let request_text = String::from_utf8_lossy(&buffer[..bytes_read]);

    println!("Request: \n{}", request_text);

    let request = parse_request(&request_text).unwrap();

    println!("Parsed request: {:?}", request);

    let (status_line, content_type, body) = match request.path.as_str() {
        "/" => (
            "HTTP/1.1 200 OK",
            "text/html; charset=utf-8",
            "<h1>Home</h1> <p>Hello from a tiny Rust Http server.</p>",
        ),
        "/hello" => (
            "HTTP/1.1 200 OK",
            "text/plain; charset=utf-8",
            "hello from Rust Http server",
        ),
        "/about" => (
            "HTTP/1.1 200 OK",
            "text/plain; charset=utf-8",
            "this server is written from zero",
        ),
        _ => ("HTTP/1.1 200 OK", "text/plain; charset=utf-8", "Not Found"),
    };

    let response = format!(
        "{}\r\nContent-Length: {}\r\nContent-type: {}\r\n\r\n{}",
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
        let mut stream = stream.unwrap();

        handle_connection(stream);
    }
}
