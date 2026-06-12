use crate::config::{
    add_server_config_command, delete_server_config_command, duplicate_server_config_command,
    handle_ipc_message, load_server_config, route_info_command, save_server_config_command,
    update_ipc_context,
};
use crate::dev_commands::dev_command;
use crate::functions::{function_snapshot_json, http_function_response};
use crate::process::launch_plan_command;
use crate::process_readiness::readiness_command;
use crate::process_supervisor::supervisor_command;
use crate::project::project_command;
use crate::proxy::proxy_command;
use crate::routes::route_command;

pub(crate) fn handle_command(command: &str) -> String {
    let parsed: serde_json::Value =
        serde_json::from_str(command).unwrap_or(serde_json::Value::Null);
    let command_type = parsed
        .get("type")
        .and_then(|value| value.as_str())
        .unwrap_or("");

    if command_type == "servers.snapshot" {
        return serde_json::json!({
            "type": "servers.snapshot",
            "payload": load_server_config(),
        })
        .to_string();
    }

    if command_type == "servers.save" {
        let payload = parsed.get("payload").unwrap_or(&serde_json::Value::Null);
        return match save_server_config_command(payload) {
            Ok(configuration) => serde_json::json!({
                "type": "servers.saved",
                "payload": configuration,
            })
            .to_string(),
            Err(message) => serde_json::json!({
                "type": "error",
                "message": message,
            })
            .to_string(),
        };
    }

    if matches!(
        command_type,
        "servers.add" | "servers.delete" | "servers.duplicate"
    ) {
        let payload = parsed.get("payload").unwrap_or(&serde_json::Value::Null);
        let result = match command_type {
            "servers.add" => add_server_config_command(payload),
            "servers.delete" => delete_server_config_command(payload),
            "servers.duplicate" => duplicate_server_config_command(payload),
            _ => unreachable!(),
        };
        return match result {
            Ok(configuration) => serde_json::json!({
                "type": command_type,
                "payload": configuration,
            })
            .to_string(),
            Err(message) => serde_json::json!({
                "type": "error",
                "message": message,
            })
            .to_string(),
        };
    }

    if command_type == "server.routeInfo" {
        let payload = parsed.get("payload").unwrap_or(&serde_json::Value::Null);
        return match route_info_command(payload) {
            Ok(info) => serde_json::json!({
                "type": "server.routeInfo",
                "payload": info,
            })
            .to_string(),
            Err(message) => serde_json::json!({
                "type": "error",
                "message": message,
            })
            .to_string(),
        };
    }

    if command_type == "ipc.handle" {
        let message = parsed.get("payload").unwrap_or(&serde_json::Value::Null);
        let context = parsed.get("context").unwrap_or(&serde_json::Value::Null);
        return handle_ipc_message(message, context).to_string();
    }

    if command_type == "ipc.context" {
        let payload = parsed.get("payload").unwrap_or(&serde_json::Value::Null);
        return update_ipc_context(payload).to_string();
    }

    if matches!(command_type, "server.start" | "server.stop") {
        let (callback, callback_context) = crate::callback_info();
        if let Some(response) = supervisor_command(
            command_type,
            parsed.get("payload").unwrap_or(&serde_json::Value::Null),
            callback,
            callback_context,
        ) {
            return response.to_string();
        }
    }

    if command_type == "server.launchPlan" {
        let payload = parsed.get("payload").unwrap_or(&serde_json::Value::Null);
        return launch_plan_command(payload).to_string();
    }

    if let Some(response) = readiness_command(
        command_type,
        parsed.get("payload").unwrap_or(&serde_json::Value::Null),
    ) {
        return response.to_string();
    }

    if let Some(response) = route_command(
        command_type,
        parsed.get("payload").unwrap_or(&serde_json::Value::Null),
    ) {
        return response.to_string();
    }

    if let Some(response) = proxy_command(
        command_type,
        parsed.get("payload").unwrap_or(&serde_json::Value::Null),
    ) {
        return response.to_string();
    }

    if let Some(response) = dev_command(
        command_type,
        parsed.get("payload").unwrap_or(&serde_json::Value::Null),
    ) {
        return response.to_string();
    }

    if let Some(response) = project_command(
        command_type,
        parsed.get("payload").unwrap_or(&serde_json::Value::Null),
    ) {
        return response.to_string();
    }

    let Some(started_at_ms) = crate::started_at_ms() else {
        return r#"{"type":"error","message":"rack core is not running"}"#.to_string();
    };

    if command_type == "state.snapshot" {
        let servers = load_server_config().servers;
        return format!(
            r#"{{"type":"state.snapshot","payload":{{"backend":"rust","started_at_ms":{},"servers":{},"functions":{}}}}}"#,
            started_at_ms,
            serde_json::to_string(&servers).unwrap_or_else(|_| "[]".to_string()),
            function_snapshot_json()
        );
    }

    if command_type == "function.http" {
        let payload = parsed.get("payload").unwrap_or(&serde_json::Value::Null);
        return http_function_response(payload).to_string();
    }

    format!(
        r#"{{"type":"ack","payload":{{"backend":"rust","command":{}}}}}"#,
        if command.is_empty() { "null" } else { command }
    )
}
