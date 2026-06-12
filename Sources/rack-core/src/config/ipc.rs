use super::models::{local_url, new_id, EnvironmentVariable, ServerConfiguration};
use super::storage::{load_server_config, save_server_config};
use serde::Deserialize;
use serde_json::Value;
use std::sync::{Mutex, OnceLock};

static IPC_CONTEXT: OnceLock<Mutex<Value>> = OnceLock::new();

fn ipc_context_storage() -> &'static Mutex<Value> {
    IPC_CONTEXT.get_or_init(|| Mutex::new(serde_json::json!({})))
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
