mod http;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

#[derive(Debug)]
struct Request {
    method: String,

    path: String,

    version: String,

    headers: Vec<Header>,

    body: Vec<u8>,
}

impl Request {
    fn body_text(&self) -> String {
        String::from_utf8_lossy(&self.body).to_string()
    }

    fn header(&self, name: &str) -> Option<&str> {
        for header in &self.headers {
            if header.name.eq_ignore_ascii_case(name) {
                return Some(header.value.as_str());
            }
        }

        None
    }
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    let marker = b"\r\n\r\n";

    buffer
        .windows(marker.len())
        .position(|window| window == marker)
}

fn parse_content_length_from_head(head_text: &str) -> Option<usize> {
    for line in head_text.lines() {
        let (name, value) = line.split_once(":")?;

        if name.trim().eq_ignore_ascii_case("Content-Length") {
            return value.trim().parse::<usize>().ok();
        }
    }

    None
}

fn read_http_request(stream: &mut TcpStream) -> Option<Vec<u8>> {
    let mut buffer = Vec::new();

    let mut temp = [0; 1024];

    let mut expected_total: Option<usize> = None;

    loop {
        let bytes_read = stream.read(&mut temp).unwrap();

        if bytes_read == 0 {
            break;
        }

        buffer.extend_from_slice(&temp[..bytes_read]);

        if expected_total.is_none() {
            if let Some(header_end) = find_header_end(&buffer) {
                let head_text = String::from_utf8_lossy(&buffer[..bytes_read]);

                let content_length = parse_content_length_from_head(&head_text).unwrap_or(0);

                let body_start = header_end + 4;

                expected_total = Some(body_start + content_length);
            }
        }

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
            let body = b"<h1>Home</h1><p>Structured HTTP server.</p>".to_vec();

            let mut response = Response::new(200, "OK", body);

            response.set_header("Content-Type", "text/html; charset=utf-8");

            response
        }

        ("GET", "/hello") => {
            let body = b"Hello from structed Rust HTTP server".to_vec();

            let mut response = Response::new(200, "OK", body);

            response.set_header("Content-type", "text/plain; charset=utf-8");

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

fn parse_request(raw: &[u8]) -> Option<Request> {
    let header_end = find_header_end(raw)?;

    let body_start = header_end + 4;

    let head_text = String::from_utf8_lossy(&raw[..header_end]);

    let body = raw[body_start..].to_vec();

    let mut lines = head_text.lines();

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
        body,
    })
}

fn handle_connection(mut stream: TcpStream) {
    let Some(raw_request) = read_http_request(&mut stream) else {
        return;
    };

    println!("Raw request:\n{}", String::from_utf8_lossy(&raw_request));

    let request = parse_request(&raw_request).unwrap();

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
