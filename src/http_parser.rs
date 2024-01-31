use std::collections::HashMap;
#[derive(Debug,PartialEq,Hash,Eq,Clone,Copy)]
pub enum HttpParseError {
    InvalidMethod,
}
#[derive(PartialEq,Debug,Hash,Eq,Clone,Copy)]
pub enum HttpMethod{
    GET,
    POST,
    PUT,
    DELETE,
}

pub struct HttpRequest{
    pub method: HttpMethod,
    pub path: String,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}
pub fn parse(request_buffer:&[u8])->Result<HttpRequest,HttpParseError>{
    let mut headers = HashMap::new();
    let mut request = String::from_utf8_lossy(request_buffer);
    let mut request_lines = request.lines();

    let mut request_line = request_lines.next().ok_or(HttpParseError::InvalidMethod)?.split_whitespace();

    let method = request_line.next().ok_or(HttpParseError::InvalidMethod)?;
    println!("method {}",method);
    let path = request_line.next().ok_or(HttpParseError::InvalidMethod)?;
    println!("path {}",path);
    let version = request_line.next().ok_or(HttpParseError::InvalidMethod)?;
    println!("version {}",version);
    for line in request_lines.clone(){
        if line.is_empty(){
            break;
        }
        let mut header_line = line.split_once(":");
        if let Some((header_name,header_value)) = header_line {

            headers.insert(header_name.trim().to_string(), header_value.trim().to_string());
        }
    }

    let body = request_lines.skip(headers.len()).skip(1).next().map(|line| line.to_string());
    let method = match method {
        "GET" => HttpMethod::GET,
        "POST" => HttpMethod::POST,
        "PUT" => HttpMethod::PUT,
        "DELETE" => HttpMethod::DELETE,
        _ => return Err(HttpParseError::InvalidMethod),
    };
    Ok(HttpRequest {
        method,
        path: path.to_string(),
        version: version.to_string(),
        headers,
        body,
    })
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
        let request_buffer = b"DELETE / HTTP/1.1\r\nHost: localhost:3000\r\nContent-Type: application/json\r\nContent-Length: 20\r\n\r\n{\"name\":\"John\"}";
        let request = parse(request_buffer).unwrap();
        assert_eq!(request.method, HttpMethod::DELETE);
        assert_eq!(request.path, "/");
        assert_eq!(request.version, "HTTP/1.1");
        assert_eq!(request.headers["Host"], "localhost:3000");
        assert_eq!(request.headers["Content-Type"], "application/json");
        assert_eq!(request.headers["Content-Length"], "20");
        assert_eq!(request.body, Some("{\"name\":\"John\"}".to_string()));
        let request_buffer = b"GET / HTTP/1.1\r\n\r\n";
        let request = parse(request_buffer).unwrap();
        assert_eq!(request.method, HttpMethod::GET);
        assert_eq!(request.path, "/");
        assert_eq!(request.version, "HTTP/1.1");
        assert_eq!(request.headers.len(), 0);
        assert_eq!(request.body, None);
    }
}


