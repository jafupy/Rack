use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct EnvironmentVariable {
    #[serde(default = "new_id")]
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) key: String,
    #[serde(default)]
    pub(crate) value: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ServerConfiguration {
    #[serde(default = "new_id")]
    pub(crate) id: String,
    #[serde(default = "default_server_name")]
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) command: String,
    #[serde(default)]
    pub(crate) arguments: String,
    #[serde(default, rename = "workingDirectory")]
    pub(crate) working_directory: String,
    #[serde(default, rename = "autoStart")]
    pub(crate) auto_start: bool,
    #[serde(default, rename = "customDomain")]
    pub(crate) custom_domain: String,
    #[serde(default)]
    pub(crate) environment: Vec<EnvironmentVariable>,
    #[serde(default)]
    pub(crate) port: Option<u16>,
    #[serde(default, rename = "portFlag")]
    pub(crate) port_flag: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct PersistedConfiguration {
    #[serde(default)]
    pub(crate) servers: Vec<ServerConfiguration>,
}

static IPC_CONTEXT: OnceLock<Mutex<Value>> = OnceLock::new();

fn ipc_context_storage() -> &'static Mutex<Value> {
    IPC_CONTEXT.get_or_init(|| Mutex::new(serde_json::json!({})))
}

pub(crate) fn load_server_config() -> PersistedConfiguration {
    migrate_server_config_if_needed();
    let path = config_path();
    let Ok(data) = std::fs::read_to_string(path) else {
        return PersistedConfiguration {
            servers: Vec::new(),
        };
    };

    serde_json::from_str(&data).unwrap_or(PersistedConfiguration {
        servers: Vec::new(),
    })
}

pub(crate) fn save_server_config_command(
    payload: &Value,
) -> Result<PersistedConfiguration, String> {
    let configuration = if payload.is_array() {
        PersistedConfiguration {
            servers: serde_json::from_value(payload.clone()).map_err(|error| error.to_string())?,
        }
    } else {
        serde_json::from_value(payload.clone()).map_err(|error| error.to_string())?
    };
    save_server_config(&configuration)?;
    Ok(configuration)
}

pub(crate) fn handle_ipc_message(message: &Value, context: &Value) -> Value {
    let result = match message.get("type").and_then(Value::as_str).unwrap_or("") {
        "register" => handle_ipc_register(message.get("payload").unwrap_or(&Value::Null), context),
        "start" => handle_ipc_start(message.get("payload").and_then(Value::as_str)),
        "stop" => handle_ipc_stop(message.get("payload").and_then(Value::as_str)),
        "remove" => handle_ipc_remove(message.get("payload").and_then(Value::as_str)),
        "list" => Ok(IpcResult {
            reply: ipc_servers_reply(context),
            action: None,
        }),
        _ => Ok(IpcResult {
            reply: serde_json::json!({"type":"error","payload":"unknown message"}),
            action: None,
        }),
    };

    match result {
        Ok(result) => serde_json::json!({
            "type": "ipc.reply",
            "payload": {
                "reply": result.reply,
                "action": result.action,
            }
        }),
        Err(message) => serde_json::json!({
            "type": "ipc.reply",
            "payload": {
                "reply": {"type":"error","payload": message},
                "action": null,
            }
        }),
    }
}

pub(crate) fn handle_ipc_message_with_current_context(message: &Value) -> Value {
    let context = current_ipc_context();
    handle_ipc_message(message, &context)
}

pub(crate) fn current_ipc_context() -> Value {
    ipc_context_storage().lock().unwrap().clone()
}

pub(crate) fn update_ipc_context(payload: &Value) -> Value {
    *ipc_context_storage().lock().unwrap() = payload.clone();
    serde_json::json!({
        "type": "ipc.context",
        "payload": payload,
    })
}

struct IpcResult {
    reply: Value,
    action: Option<Value>,
}

#[derive(Deserialize)]
struct IpcRegisterPayload {
    name: String,
    command: String,
    #[serde(rename = "workingDirectory")]
    working_directory: String,
    #[serde(default)]
    environment: std::collections::BTreeMap<String, String>,
    #[serde(default, rename = "portFlag")]
    port_flag: Option<String>,
}

fn handle_ipc_register(payload: &Value, context: &Value) -> Result<IpcResult, String> {
    let payload: IpcRegisterPayload =
        serde_json::from_value(payload.clone()).map_err(|error| error.to_string())?;
    let mut configuration = load_server_config();
    let server = ServerConfiguration {
        id: new_id(),
        name: payload.name,
        command: payload.command,
        arguments: String::new(),
        working_directory: payload.working_directory,
        auto_start: true,
        custom_domain: String::new(),
        environment: payload
            .environment
            .into_iter()
            .map(|(key, value)| EnvironmentVariable {
                id: new_id(),
                key,
                value,
            })
            .collect(),
        port: None,
        port_flag: payload.port_flag,
    };
    let url = local_url(&server, context);
    let id = server.id.clone();
    configuration.servers.push(server);
    save_server_config(&configuration)?;

    Ok(IpcResult {
        reply: serde_json::json!({
            "type": "registered",
            "payload": {
                "name": configuration.servers.last().map(|server| server.name.as_str()).unwrap_or(""),
                "url": url,
            }
        }),
        action: Some(serde_json::json!({
            "type": "start",
            "id": id,
        })),
    })
}

fn handle_ipc_start(name: Option<&str>) -> Result<IpcResult, String> {
    let name = name.ok_or_else(|| "missing server name".to_string())?;
    let configuration = load_server_config();
    let server = configuration
        .servers
        .iter()
        .find(|server| server.name == name)
        .ok_or_else(|| format!("no server named '{name}'"))?;
    Ok(IpcResult {
        reply: serde_json::json!({"type":"ok"}),
        action: Some(serde_json::json!({
            "type": "start",
            "id": server.id,
        })),
    })
}

fn handle_ipc_stop(name: Option<&str>) -> Result<IpcResult, String> {
    let name = name.ok_or_else(|| "missing server name".to_string())?;
    let configuration = load_server_config();
    let server = configuration
        .servers
        .iter()
        .find(|server| server.name == name)
        .ok_or_else(|| format!("no server named '{name}'"))?;
    Ok(IpcResult {
        reply: serde_json::json!({"type":"ok"}),
        action: Some(serde_json::json!({
            "type": "stop",
            "id": server.id,
        })),
    })
}

fn handle_ipc_remove(name: Option<&str>) -> Result<IpcResult, String> {
    let name = name.ok_or_else(|| "missing server name".to_string())?;
    let mut configuration = load_server_config();
    let index = configuration
        .servers
        .iter()
        .position(|server| server.name == name)
        .ok_or_else(|| format!("no server named '{name}'"))?;
    let server = configuration.servers.remove(index);
    save_server_config(&configuration)?;

    Ok(IpcResult {
        reply: serde_json::json!({"type":"ok"}),
        action: Some(serde_json::json!({
            "type": "remove",
            "id": server.id,
        })),
    })
}

fn ipc_servers_reply(context: &Value) -> Value {
    let configuration = load_server_config();
    let servers = configuration
        .servers
        .iter()
        .map(|server| {
            let status = status_for(context, &server.id);
            serde_json::json!({
                "name": server.name,
                "url": local_url(server, context),
                "running": status.running,
                "pid": status.pid,
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "type": "servers",
        "payload": servers,
    })
}

struct IpcStatus {
    running: bool,
    pid: Option<i32>,
}

fn status_for(context: &Value, id: &str) -> IpcStatus {
    let Some(statuses) = context.get("statuses").and_then(Value::as_array) else {
        return IpcStatus {
            running: false,
            pid: None,
        };
    };
    statuses
        .iter()
        .find(|status| status.get("id").and_then(Value::as_str) == Some(id))
        .map(|status| IpcStatus {
            running: status
                .get("running")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            pid: status
                .get("pid")
                .and_then(Value::as_i64)
                .map(|pid| pid as i32),
        })
        .unwrap_or(IpcStatus {
            running: false,
            pid: None,
        })
}

pub(crate) fn local_url(server: &ServerConfiguration, context: &Value) -> String {
    let subdomain = route_subdomain(server);
    if context
        .get("standardPortsEnabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return format!("http://{subdomain}.localhost");
    }
    let port = context
        .get("boundPort")
        .and_then(Value::as_u64)
        .unwrap_or(1355);
    format!("http://{subdomain}.localhost:{port}")
}

pub(crate) fn route_subdomain(server: &ServerConfiguration) -> String {
    let raw = if server.custom_domain.is_empty() {
        &server.name
    } else {
        &server.custom_domain
    };
    let trimmed = raw.trim().to_lowercase().replace(' ', "-");
    trimmed
        .strip_suffix(".localhost")
        .unwrap_or(&trimmed)
        .to_string()
}

fn save_server_config(configuration: &PersistedConfiguration) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let data = serde_json::to_string_pretty(configuration).map_err(|error| error.to_string())?;
    std::fs::write(path, data).map_err(|error| error.to_string())
}

fn migrate_server_config_if_needed() {
    let destination = config_path();
    if destination.is_file() {
        return;
    }

    let Some(source) = legacy_config_candidates()
        .into_iter()
        .find(|candidate| candidate.is_file())
    else {
        return;
    };

    if let Some(parent) = destination.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::copy(source, destination);
}

fn config_dir() -> PathBuf {
    home_dir().join(".config").join("rack")
}

fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

fn legacy_config_candidates() -> Vec<PathBuf> {
    vec![
        home_dir()
            .join(".config")
            .join("server-bar")
            .join("config.json"),
        home_dir()
            .join("Library")
            .join("Application Support")
            .join("ServerBar")
            .join("servers.json"),
    ]
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn new_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let process_id = std::process::id() & 0xffff;
    format!(
        "00000000-0000-4000-8000-{process_id:04x}{:08x}",
        timestamp as u32
    )
}

fn default_server_name() -> String {
    "New Server".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipc_register_persists_server_and_list_uses_status_context() {
        let _guard = crate::test_support::env_lock();
        let previous_home = std::env::var_os("HOME");
        let home = std::env::temp_dir().join(format!(
            "rack-core-config-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("HOME", &home);

        let register = serde_json::json!({
            "type": "register",
            "payload": {
                "name": "Demo App",
                "command": "npm run dev",
                "workingDirectory": "/tmp/demo",
                "environment": {"RUST_LOG": "debug"},
                "portFlag": "--port"
            }
        });
        let context = serde_json::json!({
            "boundPort": 1355,
            "standardPortsEnabled": false,
            "statuses": [],
        });

        let response = handle_ipc_message(&register, &context);
        assert_eq!(response["payload"]["reply"]["type"], "registered");
        assert_eq!(
            response["payload"]["reply"]["payload"]["url"],
            "http://demo-app.localhost:1355"
        );
        let id = response["payload"]["action"]["id"].as_str().unwrap();

        let list = handle_ipc_message(
            &serde_json::json!({"type": "list"}),
            &serde_json::json!({
                "boundPort": 1355,
                "standardPortsEnabled": false,
                "statuses": [{"id": id, "running": true, "pid": 42}],
            }),
        );
        assert_eq!(list["payload"]["reply"]["type"], "servers");
        assert_eq!(list["payload"]["reply"]["payload"][0]["name"], "Demo App");
        assert_eq!(list["payload"]["reply"]["payload"][0]["running"], true);
        assert_eq!(list["payload"]["reply"]["payload"][0]["pid"], 42);

        match previous_home {
            Some(value) => std::env::set_var("HOME", value),
            None => std::env::remove_var("HOME"),
        }
        let _ = std::fs::remove_dir_all(home);
    }
}
