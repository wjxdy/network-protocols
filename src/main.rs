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

impl Response {
    fn new(status_code: u16, reason: &str, body: Vec<u8>) -> Response {
        let mut response = Response {
            status_code,
            reason: reason.to_string(),
            headers: Vec::new(),
            body,
        };

        let content_length = response.body.len().to_string();
        response.set_header("Content-Length", &content_length);

        response
    }

    fn set_header(&mut self, name: &str, value: &str) {
        for header in &mut self.headers {
            if header.name.eq_ignore_ascii_case(name) {
                header.value = value.to_string();
                return;
            }
        }

        self.headers.push(Header {
            name: name.to_string(),
            value: value.to_string(),
        });
    }

    fn to_http_bytes(&self) -> Vec<u8> {
        let mut head = format!("HTTP/1.1 {} {}\r\n", self.status_code, self.reason);

        for header in &self.headers {
            head.push_str(&format!("{}: {}\r\n", header.name, header.value));
        }

        head.push_str("\r\n");

        let mut bytes = head.into_bytes();

        bytes.extend_from_slice(&self.body);

        bytes
    }
}

fn route(request: &Request) -> Response {
    if let Some(host) = request.header("Host") {
        println!("Host header: {}", host);
    }

    match request.path.as_str() {
        "/" => {
            let body = b"<h1>Home</h1><p>Structured HTTP server.</p>".to_vec();

            let mut response = Response::new(200, "OK", body);

            response.set_header("Content-Type", "text/html; charset=utf-8");

            response
        }

        "/hello" => {
            let body = b"Hello from structed Rust HTTP server".to_vec();

            let mut response = Response::new(200, "OK", body);

            response.set_header("Content-type", "text/plain; charset=utf-8");

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
    let mut buffer = [0; 4096];

    let bytes_read = stream.read(&mut buffer).unwrap();

    if bytes_read == 0 {
        return;
    }

    let request_text = String::from_utf8_lossy(&buffer[..bytes_read]);

    println!("Request: \n{}", request_text);

    let request = parse_request(&request_text).unwrap();

    let response = route(&request);

    let response_bytes = response.to_http_bytes();

    stream.write_all(&response_bytes).unwrap();
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    println!("Server listening on http://127.0.0.1:8080");

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();

        handle_connection(stream);
    }
}
