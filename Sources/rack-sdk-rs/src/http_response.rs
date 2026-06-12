use crate::{response, Error, Result};
use serde::Serialize;
use std::collections::BTreeMap;

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
    pub(crate) status: u16,
    pub(crate) headers: BTreeMap<String, String>,
    pub(crate) body: String,
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
