use rack::{Payload, Request, Response};
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
fn parses_builtin_payloads() {
    assert_eq!(String::from_body(b"hello").unwrap(), "hello");
    assert_eq!(().into_body().unwrap(), Vec::<u8>::new());
}

#[test]
fn validates_response_statuses() {
    assert!(Response::try_new(99, "bad").is_err());
    assert!(Response::text("ok").try_status(600).is_err());
}

#[test]
fn builds_helper_responses() {
    assert_eq!(Response::ok("ok").status, 200);
    assert_eq!(Response::created("ok").status, 201);
    assert_eq!(Response::bad_request("bad").status, 400);
    assert_eq!(Response::teapot("short").status, 418);

    let csv = Response::csv("a,b\n");
    assert_eq!(csv.status, 200);
    assert_eq!(
        csv.headers,
        [("content-type".into(), "text/csv; charset=utf-8".into())]
    );
    assert_eq!(csv.body, b"a,b\n");
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
