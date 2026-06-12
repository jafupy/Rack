use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Deserialize)]
struct HostPayload {
    #[serde(default)]
    host: Option<String>,
}

#[derive(Deserialize)]
struct RackLocalPayload {
    method: String,
    uri: String,
    #[serde(default)]
    headers: Vec<(String, String)>,
    #[serde(default)]
    body: String,
}

pub(crate) fn proxy_command(command_type: &str, payload: &Value) -> Option<Value> {
    let response = match command_type {
        "proxy.host" => host_command(payload),
        "proxy.rackLocalRequest" => rack_local_request_command(payload),
        _ => return None,
    };
    Some(response.unwrap_or_else(|message| {
        serde_json::json!({
            "type": "error",
            "message": message,
        })
    }))
}

fn host_command(payload: &Value) -> Result<Value, String> {
    let payload: HostPayload =
        serde_json::from_value(payload.clone()).map_err(|error| error.to_string())?;
    let hostname = payload.host.as_deref().and_then(rack_hostname);
    let route_name = hostname.as_deref().and_then(rack_route_name);
    let is_rack_local_host = hostname
        .as_deref()
        .is_some_and(|host| matches!(host, "rack.local" | "localhost" | "127.0.0.1" | "::1"));
    let is_loopback_candidate = hostname
        .as_deref()
        .is_some_and(|host| host.ends_with(".localhost"))
        && route_name.is_some();

    Ok(serde_json::json!({
        "type": "proxy.host",
        "payload": {
            "hostname": hostname,
            "routeName": route_name,
            "isRackLocalHost": is_rack_local_host,
            "isLoopbackCandidate": is_loopback_candidate,
        }
    }))
}

fn rack_local_request_command(payload: &Value) -> Result<Value, String> {
    let payload: RackLocalPayload =
        serde_json::from_value(payload.clone()).map_err(|error| error.to_string())?;
    let path = normalize_rack_local_path(&payload.uri);
    if path == "/" {
        return Ok(serde_json::json!({
            "type": "proxy.rackLocalRequest",
            "payload": {
                "kind": "root",
            }
        }));
    }
    if path.starts_with("/_") {
        return Ok(serde_json::json!({
            "type": "proxy.rackLocalRequest",
            "payload": {
                "kind": "reserved",
            }
        }));
    }

    Ok(serde_json::json!({
        "type": "proxy.rackLocalRequest",
        "payload": {
            "kind": "function",
            "command": {
                "type": "function.http",
                "payload": {
                    "method": payload.method,
                    "path": path,
                    "uri": payload.uri,
                    "headers": request_headers(payload.headers),
                    "body": payload.body,
                }
            }
        }
    }))
}

fn rack_hostname(host: &str) -> Option<String> {
    if host.is_empty() {
        return None;
    }
    if host.starts_with('[') {
        return host
            .trim_start_matches('[')
            .split_once(']')
            .map(|(value, _)| normalize_hostname(value));
    }
    host.split_once(':')
        .map(|(value, _)| normalize_hostname(value))
        .or_else(|| Some(normalize_hostname(host)))
}

fn rack_route_name(hostname: &str) -> Option<String> {
    let name = hostname.strip_suffix(".localhost")?;
    (!name.is_empty()).then(|| name.to_string())
}

fn normalize_hostname(value: &str) -> String {
    value.trim_end_matches('.').to_ascii_lowercase()
}

fn normalize_rack_local_path(uri: &str) -> String {
    let raw_path = uri.split_once('?').map(|(path, _)| path).unwrap_or(uri);
    let mut normalized = if raw_path.starts_with('/') {
        raw_path.to_string()
    } else {
        format!("/{raw_path}")
    };
    while normalized.len() > 1 && normalized.ends_with('/') {
        normalized.pop();
    }
    normalized
}

fn request_headers(headers: Vec<(String, String)>) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    for (name, value) in headers {
        let name = name.to_ascii_lowercase();
        result
            .entry(name)
            .and_modify(|existing| {
                *existing = format!("{existing}, {value}");
            })
            .or_insert(value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_host_headers() {
        let result =
            host_command(&serde_json::json!({"host": "Fix-Auth.MyApp.localhost:1355"})).unwrap();
        assert_eq!(result["payload"]["hostname"], "fix-auth.myapp.localhost");
        assert_eq!(result["payload"]["routeName"], "fix-auth.myapp");
        assert_eq!(result["payload"]["isRackLocalHost"], false);
        assert_eq!(result["payload"]["isLoopbackCandidate"], true);

        let result = host_command(&serde_json::json!({"host": "[::1]:1355"})).unwrap();
        assert_eq!(result["payload"]["hostname"], "::1");
        assert_eq!(result["payload"]["isRackLocalHost"], true);
    }

    #[test]
    fn builds_rack_local_function_command() {
        let result = rack_local_request_command(&serde_json::json!({
            "method": "GET",
            "uri": "assets/logo.png?size=2",
            "headers": [["Accept", "text/plain"], ["accept", "application/json"]],
            "body": "",
        }))
        .unwrap();

        assert_eq!(result["payload"]["kind"], "function");
        assert_eq!(
            result["payload"]["command"]["payload"]["path"],
            "/assets/logo.png"
        );
        assert_eq!(
            result["payload"]["command"]["payload"]["headers"]["accept"],
            "text/plain, application/json"
        );
    }
}
