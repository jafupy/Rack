use rack::{Request, Response};

#[rack::route(GET, "hello")]
fn hello(_request: Request) -> Response {
    Response::text("hello from rack")
}
