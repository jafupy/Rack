mod support;

use rack_proxy::HookEndpoint;
use support::{request, test_proxy, HttpBackend};

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
