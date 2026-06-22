use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
};

use thiserror::Error;
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
    task::JoinHandle,
};

use crate::{route_host, HostRoute, ServiceTarget, TargetTable};

const HEADER_LIMIT: usize = 64 * 1024;

#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("failed to bind proxy listener at {addr}: {source}")]
    Bind { addr: SocketAddr, source: io::Error },

    #[error("proxy task failed: {0}")]
    Task(#[from] tokio::task::JoinError),
}

#[derive(Clone)]
pub struct SharedTargets(Arc<RwLock<TargetTable>>);

impl SharedTargets {
    pub fn new(targets: TargetTable) -> Self {
        Self(Arc::new(RwLock::new(targets)))
    }

    pub fn update(&self, targets: TargetTable) {
        *self.0.write().expect("proxy target lock poisoned") = targets;
    }

    fn resolve(&self, host: &str) -> Option<ServiceTarget> {
        self.0
            .read()
            .expect("proxy target lock poisoned")
            .resolve(host)
            .cloned()
    }
}

pub struct ProxyServer {
    addr: SocketAddr,
    targets: SharedTargets,
    shutdown: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

impl ProxyServer {
    pub async fn bind(addr: SocketAddr, targets: TargetTable) -> Result<Self, ProxyError> {
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|source| ProxyError::Bind { addr, source })?;
        let addr = listener
            .local_addr()
            .map_err(|source| ProxyError::Bind { addr, source })?;
        let targets = SharedTargets::new(targets);
        let (shutdown, stop) = oneshot::channel();
        let task = tokio::spawn(run(listener, targets.clone(), stop));

        Ok(Self {
            addr,
            targets,
            shutdown: Some(shutdown),
            task,
        })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn targets(&self) -> SharedTargets {
        self.targets.clone()
    }

    pub async fn shutdown(mut self) -> Result<(), ProxyError> {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        self.task.await?;
        Ok(())
    }
}

async fn run(listener: TcpListener, targets: SharedTargets, mut stop: oneshot::Receiver<()>) {
    loop {
        tokio::select! {
            _ = &mut stop => break,
            accepted = listener.accept() => {
                let Ok((client, _)) = accepted else { continue };
                tokio::spawn(handle_client(client, targets.clone()));
            }
        }
    }
}

async fn handle_client(mut client: TcpStream, targets: SharedTargets) {
    let Ok(request) = read_headers(&mut client).await else {
        return;
    };
    let Some(host) = host_header(&request) else {
        let _ = write_response(&mut client, 400, "missing Host header").await;
        return;
    };

    let route = match route_host(host) {
        Ok(route) => route,
        Err(error) => {
            let _ = write_response(&mut client, 404, &error.to_string()).await;
            return;
        }
    };

    let HostRoute::Service { host } = route else {
        let _ = write_response(&mut client, 501, "rack.local hooks are not wired yet").await;
        return;
    };

    let Some(target) = targets.resolve(&host) else {
        let _ = write_response(&mut client, 502, "service target is not running").await;
        return;
    };

    let Ok(mut backend) = TcpStream::connect(("127.0.0.1", target.port)).await else {
        let _ = write_response(&mut client, 502, "service target is unavailable").await;
        return;
    };

    if backend.write_all(&request).await.is_err() {
        let _ = write_response(&mut client, 502, "failed to forward request").await;
        return;
    }

    let _ = tokio::io::copy_bidirectional(&mut client, &mut backend).await;
}

async fn read_headers(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut request = Vec::new();
    let mut buffer = [0; 1024];

    loop {
        let read = stream.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..read]);

        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }

        if request.len() > HEADER_LIMIT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "headers too large",
            ));
        }
    }

    Ok(request)
}

fn host_header(request: &[u8]) -> Option<&str> {
    let request = std::str::from_utf8(request).ok()?;
    request.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("host").then(|| value.trim())
    })
}

async fn write_response(stream: &mut TcpStream, status: u16, message: &str) -> io::Result<()> {
    let reason = match status {
        400 => "Bad Request",
        404 => "Not Found",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        _ => "Error",
    };
    let body = format!("{message}\n");
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-length: {}\r\ncontent-type: text/plain; charset=utf-8\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );

    stream.write_all(response.as_bytes()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[test]
    fn extracts_host_headers_case_insensitively() {
        let request = b"GET / HTTP/1.1\r\nHost: api.localhost\r\n\r\n";

        assert_eq!(host_header(request), Some("api.localhost"));
    }

    #[test]
    fn returns_none_without_host_header() {
        let request = b"GET / HTTP/1.1\r\nUser-Agent: test\r\n\r\n";

        assert_eq!(host_header(request), None);
    }

    #[tokio::test]
    async fn forwards_get_requests_to_matching_service_target() {
        let backend = TestBackend::start().await;
        let proxy = test_proxy("api", backend.port()).await;

        let response = request(
            proxy.addr(),
            "GET /hello HTTP/1.1\r\nHost: api.localhost\r\n\r\n",
        )
        .await;

        assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
        assert!(
            response.contains("\r\n\r\nGET /hello HTTP/1.1\n"),
            "{response}"
        );
        proxy.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn forwards_post_bodies_to_matching_service_target() {
        let backend = TestBackend::start().await;
        let proxy = test_proxy("api", backend.port()).await;
        let raw_request =
            "POST /submit HTTP/1.1\r\nHost: api.localhost\r\ncontent-length: 11\r\n\r\nhello world";

        let response = request(proxy.addr(), raw_request).await;

        assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
        assert!(
            response.ends_with("POST /submit HTTP/1.1\nhello world"),
            "{response}"
        );
        proxy.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn rejects_requests_without_host_header() {
        let backend = TestBackend::start().await;
        let proxy = test_proxy("api", backend.port()).await;

        let response = request(proxy.addr(), "GET / HTTP/1.1\r\n\r\n").await;

        assert!(
            response.starts_with("HTTP/1.1 400 Bad Request"),
            "{response}"
        );
        proxy.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn returns_bad_gateway_for_unknown_service_targets() {
        let backend = TestBackend::start().await;
        let proxy = test_proxy("api", backend.port()).await;

        let response = request(
            proxy.addr(),
            "GET / HTTP/1.1\r\nHost: web.localhost\r\n\r\n",
        )
        .await;

        assert!(
            response.starts_with("HTTP/1.1 502 Bad Gateway"),
            "{response}"
        );
        proxy.shutdown().await.unwrap();
    }

    async fn test_proxy(host: &str, backend_port: u16) -> ProxyServer {
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

    async fn request(addr: SocketAddr, request: &str) -> String {
        let mut stream = TcpStream::connect(addr).await.unwrap();
        stream.write_all(request.as_bytes()).await.unwrap();
        stream.shutdown().await.unwrap();

        let mut response = String::new();
        stream.read_to_string(&mut response).await.unwrap();
        response
    }

    struct TestBackend {
        port: u16,
    }

    impl TestBackend {
        async fn start() -> Self {
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

        fn port(&self) -> u16 {
            self.port
        }
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
}
