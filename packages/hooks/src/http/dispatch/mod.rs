mod registry;
mod router;

use serde::{Deserialize, Serialize};

pub use registry::{HookEndpoint, HookRegistry};
pub use router::{is_reserved_path, normalize_path, route, RouteError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookRequest {
    pub method: String,
    pub path: String,
    pub host: String,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    #[serde(default)]
    pub body: Vec<u8>,
}

impl HookRequest {
    pub fn new(
        method: impl Into<String>,
        path: impl Into<String>,
        host: impl Into<String>,
    ) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            host: host.into(),
            query: String::new(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn query(mut self, query: impl Into<String>) -> Self {
        self.query = query.into();
        self
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    pub fn header_value(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HookResponse {
    pub fn new(status: u16, body: impl Into<Vec<u8>>) -> Self {
        assert!(
            is_valid_status(status),
            "response status must be a valid HTTP status"
        );
        Self {
            status,
            headers: Vec::new(),
            body: body.into(),
        }
    }

    pub fn text(status: u16, body: impl Into<String>) -> Self {
        Self::new(status, body.into()).header("content-type", "text/plain; charset=utf-8")
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    pub fn validate(&self) -> Result<(), InvalidHookResponse> {
        is_valid_status(self.status)
            .then_some(())
            .ok_or(InvalidHookResponse::InvalidStatus(self.status))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidHookResponse {
    InvalidStatus(u16),
}

impl std::fmt::Display for InvalidHookResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidStatus(status) => {
                write!(f, "invalid HTTP status returned by hook: {status}")
            }
        }
    }
}

impl std::error::Error for InvalidHookResponse {}

pub fn dispatch(registry: &HookRegistry, request: &HookRequest) -> HookResponse {
    let endpoint = match route(registry, request) {
        Ok(Some(endpoint)) => endpoint,
        Ok(None) => return HookResponse::text(404, "hook not found\n"),
        Err(RouteError::Conflict { .. }) => {
            return HookResponse::text(409, "conflicting hook route\n")
        }
        Err(RouteError::ReservedPath(_)) => return HookResponse::text(404, "hook not found\n"),
    };

    match registry.run(&endpoint, request) {
        Ok(response) => match response.validate() {
            Ok(()) => response,
            Err(error) => HookResponse::text(500, format!("hook failed: {error}\n")),
        },
        Err(crate::RuntimeError::MissingModule) => HookResponse::text(
            501,
            format!("hook runtime is not wired yet: {}\n", endpoint.id),
        ),
        Err(error) => HookResponse::text(500, format!("hook failed: {error}\n")),
    }
}

fn is_valid_status(status: u16) -> bool {
    (100..=599).contains(&status)
}
