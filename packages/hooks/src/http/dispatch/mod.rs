mod registry;
mod router;

use serde::{Deserialize, Serialize};

pub use registry::{HookEndpoint, HookRegistry};
pub use router::route;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookRequest {
    pub method: String,
    pub path: String,
    pub host: String,
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
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HookResponse {
    pub fn text(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            headers: vec![(
                "content-type".to_string(),
                "text/plain; charset=utf-8".to_string(),
            )],
            body: body.into().into_bytes(),
        }
    }
}

pub fn dispatch(registry: &HookRegistry, request: &HookRequest) -> HookResponse {
    let Some(endpoint) = route(registry, request) else {
        return HookResponse::text(404, "hook not found\n");
    };

    match registry.run(&endpoint, request) {
        Ok(response) => response,
        Err(crate::RuntimeError::MissingModule) => HookResponse::text(
            501,
            format!("hook runtime is not wired yet: {}\n", endpoint.id),
        ),
        Err(error) => HookResponse::text(500, format!("hook failed: {error}\n")),
    }
}
