use crate::http_parser;
use crate::http_parser::{HttpMethod, HttpParseError, HttpRequest};
use crate::radix_tree::RadixTree;
use crate::thread_pool::ThreadPool;
use std::collections::HashMap;
use std::io::{Error, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, RwLock};
#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
pub enum HttpStatusCode {
    // 1xx Informational
    Continue = 100,
    SwitchingProtocols = 101,
    Processing = 102,

    // 2xx Success
    OK = 200,
    Created = 201,
    Accepted = 202,
    NonAuthoritativeInformation = 203,
    NoContent = 204,
    ResetContent = 205,
    PartialContent = 206,
    MultiStatus = 207,
    AlreadyReported = 208,
    IMUsed = 226,

    // 3xx Redirection
    MultipleChoices = 300,
    MovedPermanently = 301,
    Found = 302,
    SeeOther = 303,
    NotModified = 304,
    UseProxy = 305,
    TemporaryRedirect = 307,
    PermanentRedirect = 308,

    // 4xx Client Error
    BadRequest = 400,
    Unauthorized = 401,
    PaymentRequired = 402,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    NotAcceptable = 406,
    ProxyAuthenticationRequired = 407,
    RequestTimeout = 408,
    Conflict = 409,
    Gone = 410,
    LengthRequired = 411,
    PreconditionFailed = 412,
    PayloadTooLarge = 413,
    URITooLong = 414,
    UnsupportedMediaType = 415,
    RangeNotSatisfiable = 416,
    ExpectationFailed = 417,
    ImATeapot = 418, // April Fools' joke in RFC 2324
    MisdirectedRequest = 421,
    UnprocessableEntity = 422,
    Locked = 423,
    FailedDependency = 424,
    TooEarly = 425,
    UpgradeRequired = 426,
    PreconditionRequired = 428,
    TooManyRequests = 429,
    RequestHeaderFieldsTooLarge = 431,
    UnavailableForLegalReasons = 451,

    // 5xx Server Error
    InternalServerError = 500,
    NotImplemented = 501,
    BadGateway = 502,
    ServiceUnavailable = 503,
    GatewayTimeout = 504,
    HTTPVersionNotSupported = 505,
    VariantAlsoNegotiates = 506,
    InsufficientStorage = 507,
    LoopDetected = 508,
    NotExtended = 510,
    NetworkAuthenticationRequired = 511,
}

pub struct HttpResponse {
    status_code: HttpStatusCode,
    headers: HashMap<String, String>,
    body: Option<String>,
}
type HttpHandlerFunction = fn(HttpRequest) -> Option<HttpResponse>;
pub struct HttpServer {
    router: Arc<RwLock<HashMap<HttpMethod, RadixTree<HttpHandlerFunction>>>>,
    port: u16,
    host: String,
}

impl HttpServer {
    pub fn new(port: u16, host: String) -> HttpServer {
        HttpServer {
            router: Arc::new(RwLock::new(HashMap::new())),
            port,
            host,
        }
    }
    pub fn new_localhost(port: u16) -> HttpServer {
        Self::new(port, "127.0.0.1".to_string())
    }
    pub fn add_route(&mut self, method: HttpMethod, path: &str, handler: HttpHandlerFunction) {
        let mut router = self.router.write().unwrap();
        router
            .entry(method)
            .or_insert(RadixTree::new("/"))
            .insert(path, handler);
    }
    fn serialize_response(http_response: HttpResponse) -> Vec<u8> {
        let mut ret = String::new();
        let reason = http_response.status_code.reason_phrase();
        ret.push_str(&format!(
            "HTTP/1.1 {} {}\r\n",
            http_response.status_code as i32, reason
        ));
        for (key, value) in http_response.headers {
            ret.push_str(&format!("{}:{}\r\n", key, value));
        }
        ret.push_str("\r\n\r\n");
        if let Some(body) = http_response.body {
            ret.push_str(&body);
        }
        ret.into_bytes()
    }

    fn send_response(http_response: HttpResponse, stream: &mut TcpStream) -> Result<(), Error> {
        stream.write(&HttpServer::serialize_response(http_response))?;
        Ok(())
    }
    fn send_not_found(stream: &mut TcpStream) -> Result<(), Error> {
        Self::send_response(HttpResponse::new_not_found(), stream)
    }
    fn send_internal_error(stream: &mut TcpStream) -> Result<(), Error> {
        Self::send_response(HttpResponse::new_internal_error(), stream)
    }
    fn handle_connection(
        router: &Arc<RwLock<HashMap<HttpMethod, RadixTree<HttpHandlerFunction>>>>,
        mut stream: TcpStream,
    ) {
        let mut buffer = [0; 1024];
        stream.read(&mut buffer).unwrap();
        if let Ok(request) = http_parser::parse(&buffer) {
            if let Some(routes) = router.read().unwrap().get(&request.method) {
                if let Some(handler) = routes.get(request.path.as_str()) {
                    if let Some(result) = (*handler)(request) {
                        if let Err(_) = HttpServer::send_response(result, &mut stream) {
                            if let Err(_) = HttpServer::send_internal_error(&mut stream) {
                                println!("Error sending response"); // replace with log
                            }
                        }
                    }
                    return;
                }
            }
            if let Err(_) = HttpServer::send_not_found(&mut stream) {
                if let Err(_) = HttpServer::send_internal_error(&mut stream) {
                    println!("Error sending response"); // replace with log
                }
            }
        } else {
            if let Err(_) = HttpServer::send_internal_error(&mut stream) {
                println!("Error sending response"); // replace with log
            }
        }
    }
    pub fn listen(self) {
        println!("Listening on port {}", self.port);
        let host = self.host.clone();
        let port = self.port;
        let router = self.router.clone();
        ThreadPool::get_instance().execute(move || {
            let listener = TcpListener::bind(format!("{}:{}", host, port)).unwrap();
            let router = router.clone();
            for stream in listener.incoming() {
                if let Ok(mut stream) = stream {
                    let router = router.clone();
                    ThreadPool::get_instance().execute(move || {
                        Self::handle_connection(&router, stream);
                    });
                }
            }
        });
    }
}

impl HttpResponse {
    pub fn new(status_code: HttpStatusCode, body: Option<String>) -> HttpResponse {
        let mut ret = HttpResponse {
            status_code,
            headers: HashMap::new(),
            body: None,
        };
        ret.headers
            .insert("Access-Control-Allow-Origin".to_string(), "*".to_string());
        ret
    }
    pub fn new_from_html_file(
        status_code: HttpStatusCode,
        file_path: String,
    ) -> Result<HttpResponse, std::io::Error> {
        let file_content = std::fs::read_to_string(file_path)?;
        let body_length = file_content.len();
        let mut ret = HttpResponse {
            status_code,
            headers: HashMap::new(),
            body: Some(file_content),
        };
        ret.headers
            .insert("Content-Type".to_string(), "text/html".to_string());
        ret.headers
            .insert("Content-Length".to_string(), body_length.to_string());
        ret.headers
            .insert("Access-Control-Allow-Origin".to_string(), "*".to_string());
        Ok(ret)
    }
    pub fn new_from_json(status_code: HttpStatusCode, json: String) -> HttpResponse {
        let body_length = json.len() + 2;

        let mut ret = HttpResponse {
            status_code,
            headers: HashMap::new(),
            body: Some(json),
        };
        ret.headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        ret.headers
            .insert("Content-Length".to_string(), body_length.to_string());
        ret.headers
            .insert("Access-Control-Allow-Origin".to_string(), "*".to_string());
        ret
    }
    pub fn new_internal_error() -> HttpResponse {
        Self::new_from_html_file(
            HttpStatusCode::InternalServerError,
            "static/500.html".to_string(),
        )
        .unwrap()
    }
    pub fn new_not_found() -> HttpResponse {
        Self::new_from_html_file(HttpStatusCode::NotFound, "static/404.html".to_string()).unwrap()
    }
    pub fn ok() -> HttpResponse {
        Self::new(HttpStatusCode::OK, None)
    }
}
impl HttpStatusCode {
    fn reason_phrase(&self) -> &str {
        match self {
            HttpStatusCode::Continue => "Continue",
            HttpStatusCode::SwitchingProtocols => "Switching Protocols",
            HttpStatusCode::Processing => "Processing",
            HttpStatusCode::OK => "OK",
            HttpStatusCode::Created => "Created",
            HttpStatusCode::Accepted => "Accepted",
            HttpStatusCode::MultipleChoices => "Multiple Choices",
            HttpStatusCode::BadRequest => "Bad Request",
            HttpStatusCode::Unauthorized => "Unauthorized",
            HttpStatusCode::InternalServerError => "Internal Server Error",
            HttpStatusCode::NonAuthoritativeInformation => "Non-Authoritative Information",
            HttpStatusCode::NoContent => "No Content",
            HttpStatusCode::ResetContent => "Reset Content",
            HttpStatusCode::PartialContent => "Partial Content",
            HttpStatusCode::MultiStatus => "Multi-Status",
            HttpStatusCode::AlreadyReported => "Already Reported",
            HttpStatusCode::IMUsed => "IM Used",
            HttpStatusCode::MovedPermanently => "Moved Permanently",
            HttpStatusCode::Found => "Found",
            HttpStatusCode::SeeOther => "See Other",
            HttpStatusCode::NotModified => "Not Modified",
            HttpStatusCode::UseProxy => "Use Proxy",
            HttpStatusCode::TemporaryRedirect => "Temporary Redirect",
            HttpStatusCode::PermanentRedirect => "Permanent Redirect",
            HttpStatusCode::PaymentRequired => "Payment Required",
            HttpStatusCode::Forbidden => "Forbidden",
            HttpStatusCode::NotFound => "Not Found",
            HttpStatusCode::MethodNotAllowed => "Method Not Allowed",
            HttpStatusCode::NotAcceptable => "Not Acceptable",
            HttpStatusCode::ProxyAuthenticationRequired => "Proxy Authentication Required",
            HttpStatusCode::RequestTimeout => "Request Timeout",
            HttpStatusCode::Conflict => "Conflict",
            HttpStatusCode::Gone => "Gone",
            HttpStatusCode::LengthRequired => "Length Required",
            HttpStatusCode::PreconditionFailed => "Precondition Failed",
            HttpStatusCode::PayloadTooLarge => "Payload Too Large",
            HttpStatusCode::URITooLong => "URI Too Long",
            HttpStatusCode::UnsupportedMediaType => "Unsupported Media Type",
            HttpStatusCode::RangeNotSatisfiable => "Range Not Satisfiable",
            HttpStatusCode::ExpectationFailed => "Expectation Failed",
            HttpStatusCode::ImATeapot => "I'm a Teapot",
            HttpStatusCode::MisdirectedRequest => "Misdirected Request",
            HttpStatusCode::UnprocessableEntity => "Unprocessable Entity",
            HttpStatusCode::Locked => "Locked",
            HttpStatusCode::FailedDependency => "Failed Dependency",
            HttpStatusCode::TooEarly => "Too Early",
            HttpStatusCode::UpgradeRequired => "Upgrade Required",
            HttpStatusCode::PreconditionRequired => "Precondition Required",
            HttpStatusCode::TooManyRequests => "Too Many Requests",
            HttpStatusCode::RequestHeaderFieldsTooLarge => "Request Header Fields Too Large",
            HttpStatusCode::UnavailableForLegalReasons => "Unavailable For Legal Reasons",
            HttpStatusCode::NotImplemented => "Not Implemented",
            HttpStatusCode::BadGateway => "Bad Gateway",
            HttpStatusCode::ServiceUnavailable => "Service Unavailable",
            HttpStatusCode::GatewayTimeout => "Gateway Timeout",
            HttpStatusCode::HTTPVersionNotSupported => "HTTP Version Not Supported",
            HttpStatusCode::VariantAlsoNegotiates => "Variant Also Negotiates",
            HttpStatusCode::InsufficientStorage => "Insufficient Storage",
            HttpStatusCode::LoopDetected => "Loop Detected",
            HttpStatusCode::NotExtended => "Not Extended",
            HttpStatusCode::NetworkAuthenticationRequired => "Network Authentication Required",
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::{JsonData, JsonDeserializer, JsonObject, JsonSerializer};
    #[test]
    fn test_http_server() {
        let mut server = HttpServer::new_localhost(8080);
        server.add_route(HttpMethod::GET, "/test", |_| {
            Some(
                HttpResponse::new_from_html_file(
                    HttpStatusCode::OK,
                    "static/test.html".to_string(),
                )
                .unwrap(),
            )
        });
        ThreadPool::get_instance().execute(move || server.listen());
        let mut stream = TcpStream::connect("127.0.0.1:8080").unwrap();
        stream
            .write(b"GET /test HTTP/1.1\r\nHost: 127.0.0.1:8080\r\n\r\n")
            .unwrap();
        let mut buffer = [0; 1024];
        stream.read(&mut buffer).unwrap();
        let response = String::from_utf8(buffer.to_vec()).unwrap();
        println!("response {} ", response);
        // assert_eq!(response.path,"/test");
        // assert_eq!(response.method,HttpMethod::GET);
        // assert_eq!(response.version,"HTTP/1.1");
        // assert_eq!(response.headers.len(),2);
        // assert_eq!(response.headers.get("Content-Type").unwrap(),"text/html");
        // assert_eq!(response.headers.get("Content-Length").unwrap(),"5");
        // assert_eq!(response.body,Some("test".to_string()));
    }
    #[test]
    fn set_up_server() {
        let mut server = HttpServer::new_localhost(80);
        server.add_route(HttpMethod::GET, "/index", |_| {
            Some(
                HttpResponse::new_from_html_file(
                    HttpStatusCode::OK,
                    "static/index.html".to_string(),
                )
                .unwrap(),
            )
        });
        server.add_route(HttpMethod::GET, "/test_json", |_| {
            let mut json = JsonObject::new();
            json.insert(
                "test".to_string(),
                JsonData::from_string("test".to_string()),
            );
            let ret = JsonSerializer::serialize(json);
            println!("{}", ret);
            Some(HttpResponse::new_from_json(HttpStatusCode::OK, ret))
        });
        server.listen();
        loop {}
    }
}
