use crate::Response;

/// Create an empty `200 OK` response.
pub fn new() -> Response {
    Response::new()
}

/// Create an empty `200 OK` response.
pub fn ok() -> Response {
    Response::new()
}

/// Create an empty `201 Created` response.
pub fn created() -> Response {
    Response::new().status(201).expect("201 is valid")
}

/// Create an empty `400 Bad Request` response.
pub fn bad_request() -> Response {
    Response::new().status(400).expect("400 is valid")
}

/// Create an empty `418 I'm a teapot` response.
pub fn teapot() -> Response {
    Response::new().status(418).expect("418 is valid")
}

/// Create an empty `500 Internal Server Error` response.
pub fn server_error() -> Response {
    Response::new().status(500).expect("500 is valid")
}
