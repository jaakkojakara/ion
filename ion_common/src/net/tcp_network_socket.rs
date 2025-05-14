use std::io::{self, Write};
use std::net::{SocketAddr, TcpStream};

use super::{HttpRequest, HttpResponse};

pub struct TcpNetworkSocket {}

impl TcpNetworkSocket {
    pub fn new() -> Self {
        Self {}
    }

    pub fn send_http_request(
        &self,
        to: SocketAddr,
        request: HttpRequest,
    ) -> io::Result<HttpResponse> {
        let mut socket = TcpStream::connect(to)?;
        let request_bytes = self.http_request_to_bytes(request);
        socket.write_all(&request_bytes)?;

        todo!()
    }

    fn http_request_to_bytes(&self, request: HttpRequest) -> Vec<u8> {
        let mut request_str = String::new();
        request_str.push_str(&request.method.to_string());
        request_str.push_str(&request.url);
        request_str.push_str("HTTP/1.1\r\n");

        // Ensure Host header is present
        if !request.headers.contains_key("Host") {
            request_str.push_str(&format!("Host: {}\r\n", request.url));
        }

        // Ensure Content-Length header is present
        if request.body.len() > 0 && !request.headers.contains_key("Content-Length") {
            request_str.push_str(&format!("Content-Length: {}\r\n", request.body.len()));
        }

        // Add other headers
        for (key, value) in &request.headers {
            request_str.push_str(&format!("{}: {}\r\n", key, value));
        }

        // Add empty line after headers
        request_str.push_str("\r\n");

        let mut request_bytes = request_str.as_bytes().to_vec();
        request_bytes.extend_from_slice(&request.body);
        request_bytes
    }
}
