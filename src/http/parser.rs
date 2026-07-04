use super::Request;
use crate::Header;

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    let marker = b"\r\n\r\n";

    buffer
        .windows(marker.len())
        .position(|window| window == marker)
}

fn parse_content_length_from_head(head_text: &str) -> Option<usize> {
    for line in head_text.lines() {
        let Some((name, value)) = line.split_once(":") else {
            continue;
        };

        if name.trim().eq_ignore_ascii_case("Content-Length") {
            return value.trim().parse::<usize>().ok();
        }
    }

    None
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_header_end() {
        let raw = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
        assert_eq!(find_header_end(raw), Some(31));
    }

    #[test]
    fn parses_content_length() {
        let head = "POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: 5";
        assert_eq!(parse_content_length_from_head(head), Some(5));
    }

    #[test]
    fn parses_header_line() {
        let header = parse_header_line("Host: 127.0.0.1:8080").unwrap();
        assert_eq!(header.name, "Host");
        assert_eq!(header.value, "127.0.0.1:8080");
    }

    #[test]
    fn parse_post_request_with_body() {
        let raw = b"POST /echo HTTP/1.1\r\nHost: localhost\r\nContent-Length: 5\r\n\r\nhello";
        let request = parse_request(raw).unwrap();

        assert_eq!(request.path, "/echo");
        assert_eq!(request.method, "POST");
        assert_eq!(request.version, "HTTP/1.1");
        assert_eq!(request.header("Host"), Some("localhost"));
        assert_eq!(request.body, b"hello");
    }
}
