use crate::http::header::Header;

pub struct Response {
    pub status_code: u16,
    pub reason: String,
    pub headers: Vec<Header>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status_code: u16, reason: &str, body: Vec<u8>) -> Response {
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

    pub fn set_header(&mut self, name: &str, value: &str) {
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

    pub fn to_http_bytes(&self) -> Vec<u8> {
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
