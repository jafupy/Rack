mod support;

use support::{read_until, request, test_proxy, HttpBackend, UpgradeBackend};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

#[tokio::test]
async fn forwards_get_requests_to_matching_service_target() {
    let backend = HttpBackend::start().await;
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
    let backend = HttpBackend::start().await;
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
async fn proxies_websocket_upgrade_streams() {
    let backend = UpgradeBackend::start().await;
    let proxy = test_proxy("api", backend.port()).await;
    let mut stream = TcpStream::connect(proxy.addr()).await.unwrap();
    let request = concat!(
        "GET /socket HTTP/1.1\r\n",
        "Host: api.localhost\r\n",
        "Connection: Upgrade\r\n",
        "Upgrade: websocket\r\n",
        "\r\n"
    );

    stream.write_all(request.as_bytes()).await.unwrap();
    let response = read_until(&mut stream, b"\r\n\r\n").await;
    stream.write_all(b"ping").await.unwrap();

    let mut echo = [0; 4];
    stream.read_exact(&mut echo).await.unwrap();

    assert!(
        response.starts_with("HTTP/1.1 101 Switching Protocols"),
        "{response}"
    );
    assert_eq!(&echo, b"ping");
    proxy.shutdown().await.unwrap();
}

#[tokio::test]
async fn rejects_requests_without_host_header() {
    let backend = HttpBackend::start().await;
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
    let backend = HttpBackend::start().await;
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
