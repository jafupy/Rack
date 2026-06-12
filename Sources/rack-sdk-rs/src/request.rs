use crate::{Payload, Result};
use serde::Deserialize;
use std::collections::BTreeMap;

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
pub(crate) struct RawRequest {
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
    pub(crate) fn into_request<T: Payload>(self) -> Result<Request<T>> {
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
