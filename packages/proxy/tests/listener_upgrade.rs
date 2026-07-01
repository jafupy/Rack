mod support;

use support::{read_until, test_proxy, UpgradeBackend};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

const UPGRADE_REQUEST: &str = concat!(
    "GET /socket HTTP/1.1\r\n",
    "Host: api.localhost\r\n",
    "Connection: Upgrade\r\n",
    "Upgrade: websocket\r\n",
    "\r\n"
);

#[tokio::test]
async fn proxies_websocket_upgrade_streams() {
    let backend = UpgradeBackend::start().await;
    let proxy = test_proxy("api", backend.port()).await;
    let mut stream = TcpStream::connect(proxy.addr()).await.unwrap();

    stream.write_all(UPGRADE_REQUEST.as_bytes()).await.unwrap();
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
    let frame = [
        0x81, 0x85, 0x37, 0xfa, 0x21, 0x3d, 0x7f, 0x9f, 0x4d, 0x51, 0x58,
    ];

    stream.write_all(UPGRADE_REQUEST.as_bytes()).await.unwrap();
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
