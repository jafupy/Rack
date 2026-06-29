use rack_hooks::{dispatch, HookEndpoint, HookRegistry, HookRequest, HookResponse};

#[test]
fn dispatch_returns_not_found_without_matching_hook() {
    let registry = HookRegistry::default();
    let request = HookRequest::new("GET", "/hello", "rack.local");

    let response = dispatch(&registry, &request);

    assert_eq!(response.status, 404);
    assert_eq!(response.body, b"hook not found\n");
}

#[test]
fn hook_requests_capture_query_headers_and_body() {
    let request = HookRequest::new("POST", "/hello", "rack.local")
        .query("name=rack")
        .header("content-type", "text/plain")
        .body("hello");

    assert_eq!(request.query, "name=rack");
    assert_eq!(request.header_value("Content-Type"), Some("text/plain"));
    assert_eq!(request.body, b"hello");
}

#[test]
fn hook_response_validation_rejects_invalid_statuses() {
    let response = HookResponse {
        status: 99,
        headers: Vec::new(),
        body: Vec::new(),
    };

    assert!(response.validate().is_err());
}

#[test]
fn dispatch_matches_registered_http_hook() {
    let registry = HookRegistry::new([HookEndpoint::new("hello", "GET", "/hello")]);
    let request = HookRequest::new("get", "/hello", "rack.local");

    let response = dispatch(&registry, &request);

    assert_eq!(response.status, 501);
    assert_eq!(response.body, b"hook runtime is not wired yet: hello\n");
}

#[test]
fn registry_removes_hooks_by_id() {
    let registry = HookRegistry::new([HookEndpoint::new("hello", "GET", "/hello")]);
    registry.remove("hello");

    let response = dispatch(&registry, &HookRequest::new("GET", "/hello", "rack.local"));

    assert_eq!(response.status, 404);
}
