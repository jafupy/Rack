use super::{HookEndpoint, HookRegistry, HookRequest};

pub fn route(registry: &HookRegistry, request: &HookRequest) -> Option<HookEndpoint> {
    let method = request.method.to_ascii_uppercase();
    registry
        .endpoints()
        .into_iter()
        .find(|endpoint| endpoint.method == method && endpoint.path == request.path)
}
