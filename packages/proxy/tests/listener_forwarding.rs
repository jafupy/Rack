mod support;

use rack_proxy::TargetTable;
use support::{request, test_proxy, HttpBackend};
use tokio::time::{sleep, Duration};

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
