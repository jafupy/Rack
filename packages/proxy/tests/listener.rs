mod support;

use rack_proxy::{HookEndpoint, ServiceTarget, TargetTable};
use support::{read_until, request, test_proxy, HttpBackend, UpgradeBackend};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::{sleep, Duration},
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
async fn waits_briefly_for_starting_service_targets() {
    let backend = HttpBackend::start().await;
    let proxy =
        rack_proxy::ProxyServer::bind("127.0.0.1:0".parse().unwrap(), TargetTable::default())
            .await
            .unwrap();
    let routes = proxy.targets();
    tokio::spawn(async move {
        sleep(Duration::from_millis(100)).await;
        routes.set("api", rack_proxy::Destination::loopback(backend.port()));
    });

    let response = request(
        proxy.addr(),
        "GET /late HTTP/1.1\r\nHost: api.localhost\r\n\r\n",
    )
    .await;

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(
        response.contains("\r\n\r\nGET /late HTTP/1.1\n"),
        "{response}"
    );
    proxy.shutdown().await.unwrap();
}

#[tokio::test]
async fn returns_loop_detected_for_self_targeting_routes() {
    let proxy =
        rack_proxy::ProxyServer::bind("127.0.0.1:0".parse().unwrap(), TargetTable::default())
            .await
            .unwrap();
    proxy.targets().update(TargetTable::new([ServiceTarget {
        service_id: "api".to_string(),
        host: "api".to_string(),
        port: proxy.addr().port(),
    }]));

    let response = request(
        proxy.addr(),
        "GET /loop HTTP/1.1\r\nHost: api.localhost\r\n\r\n",
    )
    .await;

    assert!(response.starts_with("HTTP/1.1 508"), "{response}");
    assert!(
        response.ends_with("proxy loop detected: service destination points back to rack proxy\n"),
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
async fn tunnels_websocket_frame_bytes_after_upgrade() {
    let backend = UpgradeBackend::start_echo_bytes(11).await;
    let proxy = test_proxy("api", backend.port()).await;
    let mut stream = TcpStream::connect(proxy.addr()).await.unwrap();
    let request = concat!(
        "GET /socket HTTP/1.1\r\n",
        "Host: api.localhost\r\n",
        "Connection: Upgrade\r\n",
        "Upgrade: websocket\r\n",
        "\r\n"
    );
    let frame = [
        0x81, 0x85, 0x37, 0xfa, 0x21, 0x3d, 0x7f, 0x9f, 0x4d, 0x51, 0x58,
    ];

    stream.write_all(request.as_bytes()).await.unwrap();
    let response = read_until(&mut stream, b"\r\n\r\n").await;
    stream.write_all(&frame).await.unwrap();

    let mut echo = [0; 11];
    stream.read_exact(&mut echo).await.unwrap();

    assert!(
        response.starts_with("HTTP/1.1 101 Switching Protocols"),
        "{response}"
    );
    assert_eq!(echo, frame);
    proxy.shutdown().await.unwrap();
}

#[tokio::test]
async fn dispatches_rack_local_to_hooks() {
    let backend = HttpBackend::start().await;
    let proxy = test_proxy("api", backend.port()).await;
    proxy
        .hooks()
        .register(HookEndpoint::new("hello", "GET", "/hello"));

    let response = request(
        proxy.addr(),
        "GET /hello HTTP/1.1\r\nHost: rack.local\r\n\r\n",
    )
    .await;

    assert!(
        response.starts_with("HTTP/1.1 501 Not Implemented"),
        "{response}"
    );
    assert!(
        response.ends_with("hook runtime is not wired yet: hello\n"),
        "{response}"
    );
    proxy.shutdown().await.unwrap();
}

#[tokio::test]
async fn returns_not_found_for_unknown_hooks() {
    let backend = HttpBackend::start().await;
    let proxy = test_proxy("api", backend.port()).await;

    let response = request(
        proxy.addr(),
        "GET /missing HTTP/1.1\r\nHost: rack.local\r\n\r\n",
    )
    .await;

    assert!(response.starts_with("HTTP/1.1 404 Not Found"), "{response}");
    assert!(response.ends_with("hook not found\n"), "{response}");
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
