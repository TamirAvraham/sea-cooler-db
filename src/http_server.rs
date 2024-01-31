use std::collections::HashMap;
use std::io::{Error, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, RwLock};
use crate::http_parser::{HttpRequest, HttpParseError, HttpMethod};
use crate::radix_tree::RadixTree;
use crate::http_parser;
use crate::thread_pool::ThreadPool;

struct HttpResponse{
    status_code: u16,
    headers: HashMap<String, String>,
    body: Option<String>,
}
type HttpHandlerFunction= fn(HttpRequest) -> Option<HttpResponse>;
struct HttpServer{
    router:Arc<RwLock<HashMap<HttpMethod, RadixTree<Box<HttpHandlerFunction>>>>>,
    port:u16,
    host:String,
}

impl HttpServer {
    pub fn new(port:u16,host:String) -> HttpServer {
        HttpServer {
            router: Arc::new(RwLock::new(HashMap::new())),
            port,
            host,
        }
    }
    pub fn new_localhost(port:u16) -> HttpServer {
        Self::new(port, "127.0.0.1".to_string())
    }
    pub fn add_route(&mut self, method:HttpMethod, path:String, handler: Box<HttpHandlerFunction>){
        let mut router=self.router.write().unwrap();
        router.entry(method).or_insert(RadixTree::new("/")).insert(path.as_str(), handler);
    }
    fn serialize_response(http_response: HttpResponse) -> Vec<u8>{
        let mut ret=String::new();
        ret.push_str(&format!("HTTP/1.1 {}\r\n", http_response.status_code));
        for (key,value) in http_response.headers {
            ret.push_str(&format!("{}:{}\r\n", key, value));
        }
        ret.push_str("\r\n\r\n");
        if let Some(body) = http_response.body {
            ret.push_str(&body);
        }
        ret.into_bytes()
    }

    fn send_response(http_response: HttpResponse, stream: &mut TcpStream)->Result<(),Error>{
        stream.write(&HttpServer::serialize_response(http_response))?;
        Ok(())
    }
    fn send_not_found(stream: &mut TcpStream) -> Result<(), Error> {
        Self::send_response(HttpResponse::new_not_found(),stream)
    }
    fn send_internal_error(stream: &mut TcpStream) -> Result<(), Error> {
        Self::send_response(HttpResponse::new_internal_error(),stream)
    }
    fn handle_connection(router:&Arc<RwLock<HashMap<HttpMethod, RadixTree<Box<HttpHandlerFunction>>>>>,mut stream: TcpStream){
        let mut buffer = [0; 1024];
        stream.read(&mut buffer).unwrap();
        let request=http_parser::parse(&buffer);
        if let Ok(request) = request {
            if let Some(routes) = router.read().unwrap().get(&request.method) {
                if let Some(handler) = routes.get(request.path.as_str()) {
                    if let Some(result)=(*handler)(request){
                        if let Err(_) = HttpServer::send_response(result,&mut stream) {
                            if let Err(_) = HttpServer::send_internal_error(&mut stream){
                                println!("Error sending response");// replace with log
                            }
                        }
                    }
                    return;
                }
            }
            if let Err(_) = HttpServer::send_not_found(&mut stream) {
                if let Err(_) = HttpServer::send_internal_error(&mut stream){
                    println!("Error sending response");// replace with log
                }
            }
        } else {
            if let Err(_) = HttpServer::send_internal_error(&mut stream){
                println!("Error sending response");// replace with log
            }
        }
    }
    pub fn listen(self){
        println!("Listening on port {}", self.port);
        let host=self.host.clone();
        let port=self.port;
        let router =self.router.clone();
        ThreadPool::get_instance().execute(move || {
            let listener = TcpListener::bind(format!("{}:{}", host, port)).unwrap();
            let router=router.clone();
            for stream in listener.incoming() {
                if let Ok(mut stream) = stream {
                    let router=router.clone();
                    ThreadPool::get_instance().execute(move || {
                        Self::handle_connection(&router, stream);
                    });

                }
            }
        });
    }
}


impl HttpResponse{

    pub fn new(status_code:u16, body:Option<String>)->HttpResponse{
        HttpResponse{
            status_code,
            headers:HashMap::new(),
            body:None,
        }
    }
    pub fn new_from_html_file(status_code:u16, file_path:String)->Result<HttpResponse,std::io::Error> {
        let file_content=std::fs::read_to_string(file_path)?;
        let body_length=file_content.len();
        let mut ret=HttpResponse {
            status_code,
            headers: HashMap::new(),
            body: Some(file_content),
        };
        ret.headers.insert("Content-Type".to_string(), "text/html".to_string());
        ret.headers.insert("Content-Length".to_string(),body_length.to_string());
        Ok(ret)
    }
    pub fn new_from_json(status_code:u16, json:String)->HttpResponse {
        let body_length=json.len()+2;

        let mut ret=HttpResponse {
            status_code,
            headers: HashMap::new(),
            body: Some(json),
        };
        ret.headers.insert("Content-Type".to_string(), "application/json".to_string());
        ret.headers.insert("Content-Length".to_string(),body_length.to_string());

        ret
    }
    pub fn new_internal_error()->HttpResponse {
        Self::new_from_html_file(500, "static/500.html".to_string()).unwrap()
    }
    pub fn new_not_found()->HttpResponse {
        Self::new_from_html_file(404, "static/404.html".to_string()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::json::{JsonData, JsonDeserializer, JsonObject, JsonSerializer};
    use super::*;
    #[test]
    fn test_http_server() {
        let mut server = HttpServer::new_localhost(8080);
        server.add_route(HttpMethod::GET, "/test".to_string(), Box::new(|_|{
            Some(HttpResponse::new_from_html_file(200, "static/test.html".to_string()).unwrap())
        }));
        ThreadPool::get_instance().execute(move || server.listen());
        let mut stream = TcpStream::connect("127.0.0.1:8080").unwrap();
        stream.write(b"GET /test HTTP/1.1\r\nHost: 127.0.0.1:8080\r\n\r\n").unwrap();
        let mut buffer = [0; 1024];
        stream.read(&mut buffer).unwrap();
        let response=String::from_utf8(buffer.to_vec()).unwrap();
        println!("response {} ",response);
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
        server.add_route(HttpMethod::GET, "/index".to_string(), Box::new(|_|{
            Some(HttpResponse::new_from_html_file(200, "static/index.html".to_string()).unwrap())
        }));
        server.add_route(HttpMethod::GET, "/test_json".to_string(), Box::new(|_|{
            let mut json=JsonObject::new();
            json.insert("test".to_string(),JsonData::from_string("test".to_string()));
            let ret=JsonSerializer::serialize(json);
            println!("{}",ret);
            Some(HttpResponse::new_from_json(200, ret))
        }));
        server.listen();
        loop {

        }
    }
}