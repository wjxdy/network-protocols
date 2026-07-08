use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::net::TcpStream;

use crate::http::Request;
use crate::http::Response;

use crate::http::parser::parse_request;
use crate::http::parser::{find_header_end, parse_content_length_from_head};

use crate::ws::{is_websocket_upgrade, websocket_accept_key};

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
        ("POST", "/echo") => {
            let mut response = Response::new(200, "OK", request.body.clone());
            response.set_header("Content-Type", "text/plain; charset=utf-8");
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

fn write_sse_headers(stream: &mut TcpStream) -> std::io::Result<()> {
    let headers = concat!(
        "HTTP/1.1 200 OK\r\n",
        "Content-Type: text/event-stream\r\n",
        "Cache-Control: no-cache\r\n",
        "\r\n"
    );

    stream.write_all(headers.as_bytes())
}

fn sse_data(message: &str) -> String {
    format!("data: {}\n\n", message)
}

use std::thread::sleep;
use std::time::Duration;

fn handle_sse(mut stream: TcpStream) {
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

fn handle_webscoket_handshake(stream: &mut TcpStream, request: &Request) -> std::io::Result<()> {
    let key = request
        .header("Sec-WebSocket-Key")
        .expect("missing Sec-WebSocket-key");

    let accept = websocket_accept_key(key);

    let response = format!(
        concat!(
            "HTTP/1.1 101 Switching Protocols\r\n",
            "Upgrade: websocket\r\n",
            "Connection: Upgrade\r\n",
            "Sec-WebSocket-Accept: {}\r\n",
            "\r\n"
        ),
        accept
    );

    stream.write_all(response.as_bytes())
}

fn handle_connection(mut stream: TcpStream) {
    let Some(raw_request) = read_http_request(&mut stream) else {
        return;
    };

    println!("Raw request:\n{}", String::from_utf8_lossy(&raw_request));

    let request = parse_request(&raw_request).unwrap();

    if request.method == "GET" && request.path == "/events" {
        handle_sse(stream);
        return;
    }

    let response = route(&request);

    let response_bytes = response.to_http_bytes();

    stream.write_all(&response_bytes).unwrap();
}

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
