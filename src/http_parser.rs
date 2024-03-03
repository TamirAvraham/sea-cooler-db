use std::collections::HashMap;
use std::fmt::Display;

#[derive(Debug, PartialEq, Hash, Eq, Clone, Copy)]
pub enum HttpParseError {
    InvalidMethod,
}
#[derive(PartialEq, Debug, Hash, Eq, Clone, Copy)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    OPTIONS,
}

impl TryFrom<&str> for HttpMethod {
    type Error = HttpParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "GET" => Ok(HttpMethod::GET),
            "POST" => Ok(HttpMethod::POST),
            "PUT" => Ok(HttpMethod::PUT),
            "DELETE" => Ok(HttpMethod::DELETE),
            "OPTIONS" => Ok(HttpMethod::OPTIONS),
            _ => Err(HttpParseError::InvalidMethod),
        }
    }
}
impl Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            HttpMethod::GET => "GET".to_string(),
            HttpMethod::POST => "POST".to_string(),
            HttpMethod::PUT => "PUT".to_string(),
            HttpMethod::DELETE => "DELETE".to_string(),
            HttpMethod::OPTIONS => "OPTIONS".to_string(),
        };
        write!(f, "{}", str)
    }
}
pub struct HttpRequest {
    pub method: HttpMethod,
    pub path: String,
    pub version: String,
    pub params: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub cookie: Option<HashMap<String, String>>,
    pub body: Option<String>,
}
fn parse_cookie(cookie: &str) -> HashMap<String, String> {
    let mut cookies = HashMap::new();
    for cookie_str in cookie.split(";") {
        let mut cookie_parts = cookie_str.split("=");
        let key = cookie_parts.next().unwrap_or("");
        let value = cookie_parts.next().unwrap_or("");
        cookies.insert(key.trim().to_string(), value.trim().to_string());
    }
    cookies
}
fn parse_params(path: &str) -> (&str, HashMap<String, String>) {
    let mut params = HashMap::new();
    let mut path_parts = path.split("?");
    let path = path_parts.next().unwrap_or(path);
    if let Some(query_string) = path_parts.next() {
        for param in query_string.split("&") {
            let mut param_parts = param.split("=");
            let key = param_parts.next().unwrap_or("");
            let value = param_parts.next().unwrap_or("");
            params.insert(key.to_string(), value.to_string());
        }
    }
    (path, params)
}
pub fn parse(request_buffer: &[u8]) -> Result<HttpRequest, HttpParseError> {
    let mut headers = HashMap::new();
    let mut request = String::from_utf8_lossy(request_buffer);
    let mut request_lines = request.lines();

    let mut request_line = request_lines
        .next()
        .ok_or(HttpParseError::InvalidMethod)?
        .split_whitespace();

    let method = request_line.next().ok_or(HttpParseError::InvalidMethod)?;

    let (path, params) = parse_params(request_line.next().ok_or(HttpParseError::InvalidMethod)?);
    let version = request_line.next().ok_or(HttpParseError::InvalidMethod)?;
    for line in request_lines.clone() {
        if line.is_empty() {
            break;
        }
        let mut header_line = line.split_once(":");
        if let Some((header_name, header_value)) = header_line {
            headers.insert(
                header_name.trim().to_string(),
                header_value.trim().to_string(),
            );
        }
    }
    let cookie = headers.get("Cookie").map(|cookie| parse_cookie(cookie));
    let body = if let Some(body_start) = request.find("\r\n\r\n") {
        Some(request[body_start + 4..].to_string())
    } else {
        None
    }
    .map(|body| {
        if body.is_empty() {
            None
        } else {
            Some(body.trim_matches(char::from(0)).to_string())
        }
    })
    .unwrap_or(None);

    Ok(HttpRequest {
        method: method.try_into()?,
        path: path.to_string(),
        version: version.to_string(),
        params,
        headers,
        cookie,
        body,
    })
}
impl HttpRequest {
    pub fn get_param(&self, key: &str) -> Option<&str> {
        self.params.get(key).map(|value| value.as_str())
    }
    pub fn get_header(&self, key: &str) -> Option<&String> {
        self.headers.get(key)
    }
    pub fn get_cookie(&self, key: &str) -> Option<&String> {
        self.cookie.as_ref().and_then(|cookies| cookies.get(key))
    }
    pub fn get_body(&self) -> Option<&String> {
        self.body.as_ref()
    }
    pub fn body_is_json(&self) -> bool {
        self.get_header("Content-Type")
            .map(|content_type| content_type.contains("application/json"))
            .unwrap_or(false)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse() {
        let request_buffer = b"GET / HTTP/1.1\r\nHost: localhost:3000\r\n\r\n";
        let request = parse(request_buffer).unwrap();
        assert_eq!(request.method, HttpMethod::GET);
        assert_eq!(request.path, "/");
        assert_eq!(request.version, "HTTP/1.1");
        assert_eq!(request.headers["Host"], "localhost:3000");
        assert_eq!(request.body, None);
        let request_buffer = b"POST / HTTP/1.1\r\nHost: localhost:3000\r\nContent-Type: application/json\r\nContent-Length: 20\r\n\r\n{\"name\":\"John\"}";
        let request = parse(request_buffer).unwrap();
        assert_eq!(request.method, HttpMethod::POST);
        assert_eq!(request.path, "/");
        assert_eq!(request.version, "HTTP/1.1");
        assert_eq!(request.headers["Host"], "localhost:3000");
        assert_eq!(request.headers["Content-Type"], "application/json");
        assert_eq!(request.headers["Content-Length"], "20");
        assert_eq!(request.body, Some("{\"name\":\"John\"}".to_string()));
        let request_buffer = b"PUT / HTTP/1.1\r\nHost: localhost:3000\r\nContent-Type: application/json\r\nContent-Length: 20\r\n\r\n{\"name\":\"John\"}";
        let request = parse(request_buffer).unwrap();
        assert_eq!(request.method, HttpMethod::PUT);
        assert_eq!(request.path, "/");
        assert_eq!(request.version, "HTTP/1.1");
        assert_eq!(request.headers["Host"], "localhost:3000");
        assert_eq!(request.headers["Content-Type"], "application/json");
        assert_eq!(request.headers["Content-Length"], "20");
        assert_eq!(request.body, Some("{\"name\":\"John\"}".to_string()));
        let request_buffer = b"DELETE / HTTP/1.1\r\nHost: localhost:3000\r\nContent-Type: application/json\r\nContent-Length: 22\r\n\r\n{\n\"name\":\"John\"\n}";
        let request = parse(request_buffer).unwrap();
        assert_eq!(request.method, HttpMethod::DELETE);
        assert_eq!(request.path, "/");
        assert_eq!(request.version, "HTTP/1.1");
        assert_eq!(request.headers["Host"], "localhost:3000");
        assert_eq!(request.headers["Content-Type"], "application/json");
        assert_eq!(request.headers["Content-Length"], "22");
        assert_eq!(request.body, Some("{\n\"name\":\"John\"\n}".to_string()));
        let request_buffer = b"GET / HTTP/1.1\r\n\r\n";
        let request = parse(request_buffer).unwrap();
        assert_eq!(request.method, HttpMethod::GET);
        assert_eq!(request.path, "/");
        assert_eq!(request.version, "HTTP/1.1");
        assert_eq!(request.headers.len(), 0);
        assert_eq!(request.body, None);
        let request_buffer = b"GET /?name=John HTTP/1.1\r\n\r\n";
        let request = parse(request_buffer).unwrap();
        assert_eq!(request.method, HttpMethod::GET);
        assert_eq!(request.path, "/");
        assert_eq!(request.version, "HTTP/1.1");
        assert_eq!(request.params["name"], "John");
        assert_eq!(request.headers.len(), 0);
        assert_eq!(request.body, None);
        let request_buffer = b"GET /?name=John&age=30 HTTP/1.1\r\n\r\n";
        let request = parse(request_buffer).unwrap();
        assert_eq!(request.method, HttpMethod::GET);
        assert_eq!(request.path, "/");
        assert_eq!(request.version, "HTTP/1.1");
        assert_eq!(request.params["name"], "John");
        assert_eq!(request.params["age"], "30");
        assert_eq!(request.headers.len(), 0);
        assert_eq!(request.body, None);
    }
}
