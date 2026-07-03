use std::io::Read;
use std::net::TcpListener;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();

    println!("Server listening on http://127.0.0.1:8080");

    for stream in listener.incoming() {
        let mut stream = stream.unwrap();

        let mut buffer = [0; 1024];

        let bytes_read = stream.read(&mut buffer).unwrap();

        let request_text = String::from_utf8_lossy(&buffer[..bytes_read]);

        println!("Request: \n{}", request_text);
    }
}
