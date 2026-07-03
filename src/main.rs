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
