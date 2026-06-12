use super::process::run_wasm_function;
use crate::functions::manifest::{FunctionCron, FunctionRouteMatch};
use crate::functions::routing::{find_route, route_match_request};
use chrono::{DateTime, Local};

pub(crate) fn http_function_response(payload: &serde_json::Value) -> serde_json::Value {
    let method = payload
        .get("method")
        .and_then(|value| value.as_str())
        .unwrap_or("GET");
    let path = payload
        .get("path")
        .and_then(|value| value.as_str())
        .unwrap_or("/");

    match find_route(method, path) {
        Ok(route) => run_function(&route, payload),
        Err(message) => serde_json::json!({
            "type": "function.response",
            "payload": {
                "status": if message.starts_with("no function route") { 404 } else { 409 },
                "headers": { "content-type": "text/plain" },
                "body": format!("rack: {message}")
            }
        }),
    }
}

fn run_function(
    route_match: &FunctionRouteMatch,
    request: &serde_json::Value,
) -> serde_json::Value {
    let request = route_match_request(route_match, request);
    run_wasm_function(
        &route_match.route.function,
        &route_match.route.wasm_path,
        &request,
        "function.response",
    )
}

pub(in crate::functions) fn run_cron(
    cron: &FunctionCron,
    scheduled_at: DateTime<Local>,
) -> serde_json::Value {
    let request = serde_json::json!({
        "type": "schedule",
        "package": cron.package,
        "id": cron.id,
        "schedule": cron.schedule,
        "scheduled_at": scheduled_at.to_rfc3339(),
    });
    run_wasm_function(&cron.function, &cron.wasm_path, &request, "cron.response")
}
