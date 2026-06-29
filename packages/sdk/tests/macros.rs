use rack::{Request, Response};

#[rack::route(GET, "gscse")]
fn gscse(_request: Request) -> Response {
    Response::text("ok")
}

#[rack::route(GET, "fallible")]
fn fallible(_request: Request) -> rack::Result<Response> {
    Ok(Response::text("ok"))
}

#[rack::cron("every minute")]
fn tick() {}

#[test]
fn route_macro_keeps_function_callable() {
    let response = gscse(Request::new("GET", "/gscse", "rack.local"));

    assert_eq!(response.status, 200);
    assert_eq!(response.body, b"ok");
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
