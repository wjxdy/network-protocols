mod http;

fn main() {
    http::server::serve("127.0.0.1:8080");
}
