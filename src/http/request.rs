use crate::Header;

#[derive(Debug)]
pub struct Request {
    pub method: String,

    pub path: String,

    pub version: String,

    pub headers: Vec<Header>,

    pub body: Vec<u8>,
}

impl Request {
    pub fn body_text(&self) -> String {
        String::from_utf8_lossy(&self.body).to_string()
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        for header in &self.headers {
            if header.name.eq_ignore_ascii_case(name) {
                return Some(header.value.as_str());
            }
        }

        None
    }
}
