use rack::{Request, Response};

#[rack::route(GET, "gscse")]
fn gscse(_request: Request) -> Response {
    Response::text("ok")
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
fn cron_macro_keeps_function_callable() {
    tick();
}
