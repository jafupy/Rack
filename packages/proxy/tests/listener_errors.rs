mod support;

use rack_proxy::{ServiceTarget, TargetTable};
use support::{request, test_proxy, HttpBackend};

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
