mod registry;
mod router;

pub use registry::{HookEndpoint, HookRegistry};
pub use router::route;

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
    match route(registry, request) {
        Some(endpoint) => HookResponse::text(
            501,
            format!("hook runtime is not wired yet: {}\n", endpoint.id),
        ),
        None => HookResponse::text(404, "hook not found\n"),
    }
}
