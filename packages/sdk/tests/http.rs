use rack::{Request, Response};
use serde::Serialize;

#[test]
fn builds_text_responses() {
    let response = Response::text("ok").status(201).header("x-rack", "yes");

    assert_eq!(response.status, 201);
    assert_eq!(
        response.headers,
        [
            ("content-type".into(), "text/plain; charset=utf-8".into()),
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
    assert_eq!(request.query, "");
    assert!(request.headers.is_empty());
    assert!(request.body.is_empty());
}

#[test]
fn builds_rich_requests() {
    let request = Request::new("POST", "/hello", "rack.local")
        .query("name=rack")
        .header("content-type", "text/plain")
        .body("hello");

    assert_eq!(request.query, "name=rack");
    assert_eq!(request.header_value("Content-Type"), Some("text/plain"));
    assert_eq!(request.body, b"hello");
}

#[test]
fn validates_response_statuses() {
    assert!(Response::try_new(99, "bad").is_err());
    assert!(Response::text("ok").try_status(600).is_err());
}

#[test]
fn builds_json_responses() {
    #[derive(Serialize)]
    struct Payload<'a> {
        ok: bool,
        message: &'a str,
    }

    let response = Response::json(Payload {
        ok: true,
        message: "yes",
    })
    .unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(
        response.headers,
        [("content-type".into(), "application/json".into())]
    );
    assert_eq!(response.body, br#"{"ok":true,"message":"yes"}"#);
}
