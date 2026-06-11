//! Rust SDK for Rack function packages.
//!
//! Rack functions are compiled to `wasm32-wasip1`. Rack sends each invocation as
//! JSON on stdin and expects one response JSON object on stdout. This crate hides
//! that boundary behind small route and cron handler macros.
//!
//! HTTP routes usually take a [`Request`] and return a [`Response`]:
//!
//! ```ignore
//! #[rack::route]
//! fn hello(req: rack::Request) -> rack::Response {
//!     rack::log::info(format!("{} {}", req.method(), req.path()));
//!     rack::response::ok().text("hello")
//! }
//! ```
//!
//! JSON request bodies can be parsed into typed payloads:
//!
//! ```ignore
//! #[rack::payload]
//! struct Update {
//!     title: String,
//!     done: bool,
//! }
//!
//! #[rack::route]
//! fn update(req: rack::Request<Update>) -> rack::Response {
//!     rack::response::ok().json(req.body())
//! }
//! ```
//!
//! Route and cron handlers still declare `-> rack::Response`, but the generated
//! wrapper lets `?` work inside the function body. Bad request payloads become
//! `400` responses; handler errors become `500` responses.

pub use rack_macros::{cron, payload, route};
pub use serde;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::io::{self, Read};

/// Build an absolute path to a file in the function package.
///
/// Paths are resolved from the crate root, where `Cargo.toml` lives. Rack
/// function packages keep `manifest.toml` at that same root, so this points at
/// files relative to the package root:
///
/// ```ignore
/// const DATA_PATH: &str = rack::fs!("./public/data.csv");
/// ```
///
/// The macro expands at compile time using `CARGO_MANIFEST_DIR`.
#[macro_export]
macro_rules! fs {
    ($path:literal) => {
        concat!(env!("CARGO_MANIFEST_DIR"), "/", $path)
    };
}

/// A type that can cross the Rack request/response body boundary.
///
/// `#[rack::payload]` implements this trait for JSON structs. The SDK also
/// provides implementations for `()` and `String`.
pub trait Payload: Sized {
    /// Build this payload from the raw HTTP request body bytes.
    fn from_body(body: &[u8]) -> Result<Self>;

    /// Serialize this payload into raw response body bytes.
    fn into_body(self) -> Result<Vec<u8>>;
}

impl Payload for () {
    fn from_body(_: &[u8]) -> Result<Self> {
        Ok(())
    }

    fn into_body(self) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }
}

impl Payload for String {
    fn from_body(body: &[u8]) -> Result<Self> {
        String::from_utf8(body.to_vec()).map_err(|error| Error::new(error.to_string()))
    }

    fn into_body(self) -> Result<Vec<u8>> {
        Ok(self.into_bytes())
    }
}

/// Parsed HTTP request delivered to a `#[rack::route]` handler.
///
/// `T` is the parsed request body type. Use `Request` or `Request<()>` for
/// routes that do not need a body, `Request<String>` for raw text bodies, and
/// `Request<MyPayload>` for JSON structs marked with `#[rack::payload]`.
pub struct Request<T = ()> {
    body: T,
    meta: RequestMeta,
}

impl<T> Request<T> {
    /// Return the parsed request body.
    pub fn body(&self) -> &T {
        &self.body
    }

    /// Consume the request and return the parsed body.
    pub fn into_body(self) -> T {
        self.body
    }

    /// Return the HTTP method Rack matched for this request.
    pub fn method(&self) -> &str {
        &self.meta.method
    }

    /// Return the normalized request path.
    pub fn path(&self) -> &str {
        &self.meta.path
    }

    /// Return the full request URI, including query string when present.
    pub fn uri(&self) -> &str {
        &self.meta.uri
    }

    /// Return all request headers.
    ///
    /// Header names are lowercased by Rack. Duplicate headers are joined by the
    /// runtime before the request reaches the function.
    pub fn headers(&self) -> &BTreeMap<String, String> {
        &self.meta.headers
    }

    /// Return one request header by name.
    ///
    /// The lookup is case-insensitive because the provided name is lowercased
    /// before lookup.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.meta
            .headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }

    /// Return the manifest package name for the matched route.
    pub fn package(&self) -> Option<&str> {
        self.meta.route.as_ref().map(|route| route.package.as_str())
    }

    /// Return the manifest route id that matched this request.
    pub fn route_id(&self) -> Option<&str> {
        self.meta.route.as_ref().map(|route| route.id.as_str())
    }

    /// Return the manifest route pattern, including glob syntax when used.
    pub fn route_pattern(&self) -> Option<&str> {
        self.meta.route.as_ref().map(|route| route.pattern.as_str())
    }

    /// Return the normalized path that matched the route pattern.
    pub fn matched_path(&self) -> Option<&str> {
        self.meta
            .route
            .as_ref()
            .map(|route| route.matched_path.as_str())
    }
}

/// Host-provided route request metadata.
#[derive(Debug, Deserialize)]
pub struct RequestMeta {
    method: String,
    path: String,
    #[serde(default)]
    uri: String,
    #[serde(default)]
    headers: BTreeMap<String, String>,
    #[serde(default)]
    route: Option<RouteMeta>,
}

#[derive(Debug, Deserialize)]
struct RawRequest {
    method: String,
    path: String,
    #[serde(default)]
    uri: String,
    #[serde(default)]
    headers: BTreeMap<String, String>,
    #[serde(default)]
    body: String,
    #[serde(default)]
    route: Option<RouteMeta>,
}

impl RawRequest {
    fn into_request<T: Payload>(self) -> Result<Request<T>> {
        Ok(Request {
            body: T::from_body(self.body.as_bytes())?,
            meta: RequestMeta {
                method: self.method,
                path: self.path,
                uri: self.uri,
                headers: self.headers,
                route: self.route,
            },
        })
    }
}

#[derive(Debug, Deserialize)]
struct RouteMeta {
    package: String,
    id: String,
    #[allow(dead_code)]
    path: String,
    pattern: String,
    #[allow(dead_code)]
    method: String,
    #[allow(dead_code)]
    function: String,
    #[allow(dead_code)]
    is_glob: bool,
    matched_path: String,
}

/// Scheduled invocation metadata delivered to a `#[rack::cron]` handler.
#[derive(Debug, Deserialize)]
pub struct CronEvent {
    /// Manifest package name.
    pub package: String,
    /// Manifest cron id.
    pub id: String,
    /// Manifest schedule string.
    pub schedule: String,
    /// Timestamp for the scheduled run in RFC 3339 format.
    pub scheduled_at: String,
}

/// Error type used by SDK helpers and macro-generated wrappers.
///
/// Route and cron handlers normally return [`Response`] directly. Helper
/// functions can return `rack::Result<T>` so `?` can bubble failures into the
/// generated wrapper.
#[derive(Debug)]
pub struct Error {
    message: String,
}

/// SDK result type.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Create an SDK error from a displayable message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<String> for Error {
    fn from(error: String) -> Self {
        Self::new(error)
    }
}

impl From<&str> for Error {
    fn from(error: &str) -> Self {
        Self::new(error)
    }
}

/// HTTP response returned by route and cron handlers.
///
/// Build responses through [`response`] constructors:
///
/// ```ignore
/// rack::response::ok()
///     .header("x-rack", "gcse")
///     .html("<h1>Hello</h1>")
/// ```
pub struct Response {
    status: u16,
    headers: BTreeMap<String, String>,
    body: String,
}

impl Response {
    /// Create an empty `200 OK` response.
    pub fn new() -> Self {
        Self {
            status: 200,
            headers: BTreeMap::new(),
            body: String::new(),
        }
    }

    /// Set a custom HTTP status code.
    ///
    /// This is the escape hatch for statuses that do not have a named
    /// constructor. Codes outside `100..=599` return an error.
    pub fn status(mut self, status: u16) -> Result<Self> {
        if !(100..=599).contains(&status) {
            return Err(Error::new(format!("invalid HTTP status {status}")));
        }

        self.status = status;
        Ok(self)
    }

    /// Add or replace a response header.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    /// Set a `text/plain; charset=utf-8` response body.
    pub fn text(self, body: impl Into<String>) -> Self {
        self.with_body("text/plain; charset=utf-8", body.into())
    }

    /// Set a `text/html; charset=utf-8` response body.
    pub fn html(self, body: impl Into<String>) -> Self {
        self.with_body("text/html; charset=utf-8", body.into())
    }

    /// Set a `text/csv; charset=utf-8` response body.
    pub fn csv(self, body: impl Into<String>) -> Self {
        self.with_body("text/csv; charset=utf-8", body.into())
    }

    /// Serialize a JSON response body.
    ///
    /// Serialization failures are converted into a `500` response.
    pub fn json<T: Serialize>(self, body: &T) -> Self {
        match serde_json::to_string(body) {
            Ok(body) => self.with_body("application/json; charset=utf-8", body),
            Err(error) => response::server_error().text(error.to_string()),
        }
    }

    /// Set an `application/octet-stream` response body.
    ///
    /// Rack response bodies are strings today, so non-UTF-8 bytes are converted
    /// lossily.
    pub fn bytes(self, body: impl Into<Vec<u8>>) -> Self {
        let body = String::from_utf8_lossy(&body.into()).to_string();
        self.with_body("application/octet-stream", body)
    }

    fn with_body(mut self, content_type: &str, body: String) -> Self {
        self.headers
            .entry("content-type".to_string())
            .or_insert_with(|| content_type.to_string());
        self.body = body;
        self
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::new()
    }
}

/// Response builder constructors.
pub mod response {
    use super::Response;

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
}

/// Function logging helpers.
///
/// Logs are written as structured JSON to stderr. Rack captures them and stores
/// daily JSONL files under `~/.rack/logs/functions/...`.
pub mod log {
    use serde_json::json;
    use std::fmt;

    /// Write an info-level function log line.
    pub fn info(message: impl fmt::Display) {
        write("info", message);
    }

    /// Write a warning-level function log line.
    pub fn warn(message: impl fmt::Display) {
        write("warn", message);
    }

    /// Write an error-level function log line.
    pub fn error(message: impl fmt::Display) {
        write("error", message);
    }

    fn write(level: &str, message: impl fmt::Display) {
        eprintln!(
            "{}",
            json!({
                "rack_log": true,
                "level": level,
                "message": message.to_string(),
            })
        );
    }
}

pub mod __private {
    use super::*;

    pub type HandlerResult<T> = std::result::Result<T, Error>;

    pub fn payload_from_json<T: DeserializeOwned>(body: &[u8]) -> Result<T> {
        serde_json::from_slice(body).map_err(Error::from)
    }

    pub fn payload_to_json<T: Serialize>(value: T) -> Result<Vec<u8>> {
        serde_json::to_vec(&value).map_err(Error::from)
    }

    pub fn run_route<T, F>(handler: F)
    where
        T: Payload,
        F: FnOnce(Request<T>) -> HandlerResult<Response>,
    {
        let response = read_stdin()
            .and_then(|stdin| serde_json::from_str::<RawRequest>(&stdin).map_err(Error::from))
            .and_then(RawRequest::into_request)
            .map_err(|error| response::bad_request().text(error.to_string()))
            .and_then(|request| {
                handler(request).map_err(|error| response::server_error().text(error.to_string()))
            })
            .unwrap_or_else(|response| response);

        write_response(response);
    }

    pub fn run_route_empty<F>(handler: F)
    where
        F: FnOnce() -> HandlerResult<Response>,
    {
        let response = read_stdin()
            .and_then(|_| handler())
            .unwrap_or_else(|error| response::server_error().text(error.to_string()));

        write_response(response);
    }

    pub fn run_cron<F>(handler: F)
    where
        F: FnOnce(CronEvent) -> HandlerResult<Response>,
    {
        let response = read_stdin()
            .and_then(|stdin| serde_json::from_str::<CronEvent>(&stdin).map_err(Error::from))
            .and_then(|event| handler(event))
            .unwrap_or_else(|error| response::server_error().text(error.to_string()));

        write_response(response);
    }

    pub fn run_cron_empty<F>(handler: F)
    where
        F: FnOnce() -> HandlerResult<Response>,
    {
        let response = read_stdin()
            .and_then(|_| handler())
            .unwrap_or_else(|error| response::server_error().text(error.to_string()));

        write_response(response);
    }

    fn read_stdin() -> Result<String> {
        let mut stdin = String::new();
        io::stdin().read_to_string(&mut stdin)?;
        Ok(stdin)
    }

    fn write_response(response: Response) {
        println!(
            "{}",
            serde_json::json!({
                "status": response.status,
                "headers": response.headers,
                "body": response.body,
            })
        );
    }
}
