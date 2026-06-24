use std::net::SocketAddr;

use rack_proxy::{ProxyServer, ServiceTarget, TargetTable};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

pub async fn test_proxy(host: &str, backend_port: u16) -> ProxyServer {
    ProxyServer::bind(
        "127.0.0.1:0".parse().unwrap(),
        TargetTable::new([ServiceTarget {
            service_id: host.to_string(),
            host: host.to_string(),
            port: backend_port,
        }]),
    )
    .await
    .unwrap()
}

pub async fn request(addr: SocketAddr, request: &str) -> String {
    let mut stream = TcpStream::connect(addr).await.unwrap();
    stream.write_all(request.as_bytes()).await.unwrap();
    read_http_response(&mut stream).await
}

pub async fn read_until(stream: &mut TcpStream, needle: &[u8]) -> String {
    let mut data = Vec::new();
    let mut buffer = [0; 1];

    while stream.read_exact(&mut buffer).await.is_ok() {
        data.push(buffer[0]);
        if data.ends_with(needle) {
            break;
        }
    }

    String::from_utf8(data).unwrap()
}

pub struct HttpBackend {
    port: u16,
}

impl HttpBackend {
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let request = read_http_request(&mut stream).await;
                    let response = format!(
                        "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                        request.len(),
                        request
                    );
                    let _ = stream.write_all(response.as_bytes()).await;
                });
            }
        });
        Self { port }
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

pub struct UpgradeBackend {
    port: u16,
}

impl UpgradeBackend {
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            while let Ok((mut stream, _)) = listener.accept().await {
                tokio::spawn(async move {
                    let _ = read_until(&mut stream, b"\r\n\r\n").await;
                    let response = concat!(
                        "HTTP/1.1 101 Switching Protocols\r\n",
                        "connection: upgrade\r\n",
                        "upgrade: websocket\r\n",
                        "\r\n"
                    );
                    if stream.write_all(response.as_bytes()).await.is_ok() {
                        echo_four_bytes(&mut stream).await;
                    }
                });
            }
        });
        Self { port }
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

async fn read_http_response(stream: &mut TcpStream) -> String {
    let mut response = read_until(stream, b"\r\n\r\n").await.into_bytes();
    let content_length = parse_content_length(&response).unwrap_or(0);
    let mut body = vec![0; content_length];
    stream.read_exact(&mut body).await.unwrap();
    response.extend_from_slice(&body);
    String::from_utf8(response).unwrap()
}

async fn read_http_request(stream: &mut TcpStream) -> String {
    let mut request = Vec::new();
    let mut buffer = [0; 1024];
    let mut content_length = None;

    loop {
        let read = stream.read(&mut buffer).await.unwrap();
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..read]);

        if let Some(header_end) = header_end(&request) {
            content_length =
                content_length.or_else(|| parse_content_length(&request[..header_end]));
            let body_read = request.len() - header_end - 4;
            if body_read >= content_length.unwrap_or(0) {
                break;
            }
        }
    }

    let request = String::from_utf8(request).unwrap();
    let (head, body) = request.split_once("\r\n\r\n").unwrap();
    let request_line = head.lines().next().unwrap();
    format!("{}\n{}", request_line, body)
}

async fn echo_four_bytes(stream: &mut TcpStream) {
    let mut payload = [0; 4];
    if stream.read_exact(&mut payload).await.is_ok() {
        let _ = stream.write_all(&payload).await;
    }
}

fn header_end(request: &[u8]) -> Option<usize> {
    request.windows(4).position(|window| window == b"\r\n\r\n")
}

fn parse_content_length(headers: &[u8]) -> Option<usize> {
    let headers = std::str::from_utf8(headers).ok()?;
    headers.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("content-length")
            .then(|| value.trim().parse().ok())?
    })
}
