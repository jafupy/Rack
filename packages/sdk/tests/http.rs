use rack::{Request, Response};

#[test]
fn builds_text_responses() {
    let response = Response::text("ok").status(201).header("x-rack", "yes");

    assert_eq!(response.status, 201);
    assert_eq!(
        response.headers,
        [
            ("content-type".into(), "text/plain".into()),
            ("x-rack".into(), "yes".into()),
        ]
    );
    assert_eq!(response.body, b"ok");
}

#[test]
fn builds_requests() {
    let request = Request::new("GET", "/hello", "rack.local");

    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/hello");
    assert_eq!(request.host, "rack.local");
}
