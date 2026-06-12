use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Route {
    name: String,
    socket_path: String,
    tcp_port: u16,
    working_directory: String,
    added_at: Value,
}

static ROUTES: OnceLock<Mutex<BTreeMap<String, Route>>> = OnceLock::new();

fn routes() -> &'static Mutex<BTreeMap<String, Route>> {
    ROUTES.get_or_init(|| Mutex::new(load_routes()))
}

pub(crate) fn register_pending_route(name: &str, working_directory: &str) -> Result<(), String> {
    let route = Route {
        name: name.to_string(),
        socket_path: String::new(),
        tcp_port: 0,
        working_directory: working_directory.to_string(),
        added_at: serde_json::json!(unix_timestamp_ms()),
    };
    let mut guard = routes().lock().unwrap();
    guard.insert(route.name.clone(), route);
    persist_routes(&guard)
}

pub(crate) fn update_route_port(name: &str, tcp_port: u16) -> Result<(), String> {
    let mut guard = routes().lock().unwrap();
    if let Some(route) = guard.get_mut(name) {
        route.tcp_port = tcp_port;
        persist_routes(&guard)?;
    }
    Ok(())
}

pub(crate) fn update_route_socket_path(name: &str, socket_path: &str) -> Result<(), String> {
    let mut guard = routes().lock().unwrap();
    if let Some(route) = guard.get_mut(name) {
        route.socket_path = socket_path.to_string();
        persist_routes(&guard)?;
    }
    Ok(())
}

pub(crate) fn unregister_route(name: &str) -> Result<(), String> {
    let mut guard = routes().lock().unwrap();
    guard.remove(name);
    persist_routes(&guard)
}

pub(crate) fn route_command(command_type: &str, payload: &Value) -> Option<Value> {
    let response = match command_type {
        "routes.register" => register(payload),
        "routes.updatePort" => update_port(payload),
        "routes.updateSocketPath" => update_socket_path(payload),
        "routes.unregister" => unregister(payload),
        "routes.resolve" => resolve(payload),
        "routes.list" => list(),
        "routes.clear" => clear(),
        _ => return None,
    };
    Some(response.unwrap_or_else(|message| {
        serde_json::json!({
            "type": "error",
            "message": message,
        })
    }))
}

fn register(payload: &Value) -> Result<Value, String> {
    let route: Route =
        serde_json::from_value(payload.clone()).map_err(|error| error.to_string())?;
    {
        let mut guard = routes().lock().unwrap();
        guard.insert(route.name.clone(), route.clone());
        persist_routes(&guard)?;
    }
    Ok(serde_json::json!({
        "type": "routes.registered",
        "payload": route,
    }))
}

fn update_port(payload: &Value) -> Result<Value, String> {
    let name = payload
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing route name".to_string())?;
    let tcp_port = payload
        .get("tcpPort")
        .and_then(Value::as_u64)
        .ok_or_else(|| "missing tcpPort".to_string())? as u16;
    update(name, |route| route.tcp_port = tcp_port)
}

fn update_socket_path(payload: &Value) -> Result<Value, String> {
    let name = payload
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing route name".to_string())?;
    let socket_path = payload
        .get("socketPath")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing socketPath".to_string())?
        .to_string();
    update(name, |route| route.socket_path = socket_path)
}

fn update(name: &str, mutate: impl FnOnce(&mut Route)) -> Result<Value, String> {
    let route = {
        let mut guard = routes().lock().unwrap();
        let Some(route) = guard.get_mut(name) else {
            return Ok(serde_json::json!({
                "type": "routes.updated",
                "payload": null,
            }));
        };
        mutate(route);
        let route = route.clone();
        persist_routes(&guard)?;
        route
    };
    Ok(serde_json::json!({
        "type": "routes.updated",
        "payload": route,
    }))
}

fn unregister(payload: &Value) -> Result<Value, String> {
    let name = payload
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing route name".to_string())?;
    {
        let mut guard = routes().lock().unwrap();
        guard.remove(name);
        persist_routes(&guard)?;
    }
    Ok(serde_json::json!({
        "type": "routes.unregistered",
        "payload": {"name": name},
    }))
}

fn resolve(payload: &Value) -> Result<Value, String> {
    let name = payload
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing route name".to_string())?;
    let route = resolve_name(name);
    Ok(serde_json::json!({
        "type": "routes.resolved",
        "payload": route,
    }))
}

fn list() -> Result<Value, String> {
    let values = routes()
        .lock()
        .unwrap()
        .values()
        .cloned()
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "type": "routes.list",
        "payload": values,
    }))
}

fn clear() -> Result<Value, String> {
    {
        let mut guard = routes().lock().unwrap();
        guard.clear();
        persist_routes(&guard)?;
    }
    Ok(serde_json::json!({
        "type": "routes.cleared",
        "payload": [],
    }))
}

fn resolve_name(name: &str) -> Option<Route> {
    let guard = routes().lock().unwrap();
    if let Some(route) = guard.get(name) {
        return Some(route.clone());
    }

    let parts = name.split('.').collect::<Vec<_>>();
    if parts.len() <= 1 {
        return None;
    }
    let base = parts[1..].join(".");
    guard.get(&base).cloned()
}

fn load_routes() -> BTreeMap<String, Route> {
    let path = routes_path();
    let Ok(data) = std::fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    serde_json::from_str(&data).unwrap_or_default()
}

fn persist_routes(routes: &BTreeMap<String, Route>) -> Result<(), String> {
    let path = routes_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let data = serde_json::to_string(routes).map_err(|error| error.to_string())?;
    std::fs::write(path, data).map_err(|error| error.to_string())
}

fn routes_path() -> PathBuf {
    home_dir().join(".config").join("rack").join("routes.json")
}

fn unix_timestamp_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn resolves_exact_and_base_subdomain_routes() {
        let _guard = crate::test_support::env_lock();
        let previous_home = std::env::var_os("HOME");
        let home = std::env::temp_dir().join(format!(
            "rack-core-routes-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("HOME", &home);

        let _ = route_command("routes.clear", &serde_json::json!({}));
        route_command(
            "routes.register",
            &serde_json::json!({
                "name": "myapp",
                "socketPath": "/tmp/rack/myapp.sock",
                "tcpPort": 0,
                "workingDirectory": "/tmp/myapp",
                "addedAt": 803520000.0,
            }),
        )
        .unwrap();

        let exact = route_command("routes.resolve", &serde_json::json!({"name": "myapp"})).unwrap();
        assert_eq!(exact["payload"]["socketPath"], "/tmp/rack/myapp.sock");

        let fallback = route_command(
            "routes.resolve",
            &serde_json::json!({"name": "fix-auth.myapp"}),
        )
        .unwrap();
        assert_eq!(fallback["payload"]["name"], "myapp");

        match previous_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
        let _ = std::fs::remove_dir_all(home);
    }
}
