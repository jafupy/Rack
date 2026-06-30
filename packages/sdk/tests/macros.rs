use rack::{CronEvent, Request, Response};

#[rack::route(GET, "gscse")]
fn gscse(_request: Request) -> Response {
    Response::text("ok")
}

#[rack::route(GET, "fallible")]
fn fallible(_request: Request) -> rack::Result<Response> {
    Ok(Response::text("ok"))
}

#[rack::payload]
struct Message {
    text: String,
}

#[rack::route(POST, "typed")]
fn typed_route(request: Request<Message>) -> Response {
    Response::text(request.payload().text.clone())
}

#[rack::route(GET, "empty")]
fn empty_route() -> Response {
    Response::text("ok")
}

#[rack::cron("every minute")]
fn tick() {}

#[rack::cron("weekdays at 9:30am")]
fn tick_with_event(_event: CronEvent) {}

#[test]
fn route_macro_keeps_function_callable() {
    let response = gscse(Request::new("GET", "/gscse", "rack.local"));

    assert_eq!(response.status, 200);
    assert_eq!(response.body, b"ok");
}

#[test]
fn payload_macro_derives_json_payload() {
    let message = Message { text: "hi".into() };
    let body = <Message as rack::Payload>::into_body(message).unwrap();
    let decoded = <Message as rack::Payload>::from_body(&body).unwrap();

    assert_eq!(decoded.text, "hi");
}

#[test]
fn route_macro_accepts_typed_and_empty_handlers() {
    let request = Request::new("POST", "/typed", "rack.local")
        .body(r#"{"text":"yo"}"#)
        .parse_payload()
        .unwrap();
    let response = typed_route(request);

    assert_eq!(response.body, b"yo");
    assert_eq!(empty_route().body, b"ok");
}

#[test]
fn route_macro_accepts_result_response_handlers() {
    let response = fallible(Request::new("GET", "/fallible", "rack.local")).unwrap();

    assert_eq!(response.status, 200);
    assert_eq!(response.body, b"ok");
}

#[test]
fn cron_macro_keeps_function_callable() {
    tick();
}

#[test]
fn cron_macro_accepts_event_handlers() {
    tick_with_event(CronEvent {
        package: "demo".into(),
        hook: "tick_with_event".into(),
        schedule: "weekdays at 9:30am".into(),
        scheduled_at_unix: 42,
    });
}
