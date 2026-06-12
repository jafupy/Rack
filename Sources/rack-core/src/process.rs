use crate::config::{route_subdomain, ServerConfiguration};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

#[derive(Deserialize)]
pub(crate) struct LaunchPlanRequest {
    pub(crate) config: ServerConfiguration,
    #[serde(default)]
    pub(crate) context: LaunchContext,
}

#[derive(Default, Deserialize)]
pub(crate) struct LaunchContext {
    #[serde(default, rename = "bridgePath")]
    pub(crate) bridge_path: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LaunchPlan {
    pub(crate) subdomain: String,
    pub(crate) socket_path: String,
    pub(crate) port: u16,
    pub(crate) use_bridge: bool,
    pub(crate) executable: String,
    pub(crate) arguments: Vec<String>,
    pub(crate) working_directory: String,
    pub(crate) environment: BTreeMap<String, String>,
    pub(crate) launch_description: String,
}

pub(crate) fn launch_plan_command(payload: &Value) -> Value {
    match launch_plan(payload) {
        Ok(plan) => serde_json::json!({
            "type": "server.launchPlan",
            "payload": plan,
        }),
        Err(message) => serde_json::json!({
            "type": "error",
            "message": message,
        }),
    }
}

pub(crate) fn launch_plan(payload: &Value) -> Result<LaunchPlan, String> {
    let request: LaunchPlanRequest =
        serde_json::from_value(payload.clone()).map_err(|error| error.to_string())?;
    if request.config.command.trim().is_empty() {
        return Err("Missing command".to_string());
    }

    let subdomain = route_subdomain(&request.config);
    let port = request
        .config
        .port
        .unwrap_or_else(crate::process_readiness::allocate_port);
    let socket_path = socket_path(&subdomain);
    let bridge_path = request.context.bridge_path.filter(|path| !path.is_empty());
    let inner_tokens = inner_command_tokens(&request.config, port);
    let mut environment = login_shell_environment();
    for (key, value) in std::env::vars() {
        environment.entry(key).or_insert(value);
    }
    environment.insert("TERM".to_string(), "xterm-256color".to_string());
    environment.insert("COLORTERM".to_string(), "truecolor".to_string());
    environment.insert("FORCE_COLOR".to_string(), "1".to_string());
    environment.insert("CLICOLOR_FORCE".to_string(), "1".to_string());
    for variable in &request.config.environment {
        if !variable.key.is_empty() {
            environment.insert(variable.key.clone(), variable.value.clone());
        }
    }

    let (executable, arguments, use_bridge, launch_description) =
        if let Some(bridge_path) = bridge_path {
            (
                bridge_path,
                bridge_arguments(&socket_path, port, &inner_tokens),
                true,
                format!(
                    "rack-bridge --socket {socket_path} --port {port} -- {}",
                    inner_tokens.join(" ")
                ),
            )
        } else {
            environment.insert("PORT".to_string(), port.to_string());
            environment.insert("HOST".to_string(), "127.0.0.1".to_string());
            let command_line = format!(
                "clear; exec {}",
                inner_tokens
                    .iter()
                    .map(|token| shell_escape(token))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            (
                "/bin/zsh".to_string(),
                vec![
                    "-i".to_string(),
                    "-l".to_string(),
                    "-c".to_string(),
                    command_line,
                ],
                false,
                inner_tokens.join(" "),
            )
        };

    Ok(LaunchPlan {
        subdomain,
        socket_path,
        port,
        use_bridge,
        executable,
        arguments,
        working_directory: normalize_path(&request.config.working_directory),
        environment,
        launch_description,
    })
}

fn socket_path(subdomain: &str) -> String {
    format!("/tmp/rack/{subdomain}.sock")
}

fn inner_command_tokens(config: &ServerConfiguration, port: u16) -> Vec<String> {
    let mut tokens = split_words(&config.command);
    tokens.extend(split_words(&config.arguments));
    if let Some(flag) = &config.port_flag {
        if !flag.is_empty() {
            tokens.push(flag.clone());
            tokens.push(port.to_string());
        }
    }
    tokens
}

fn bridge_arguments(socket_path: &str, port: u16, inner_tokens: &[String]) -> Vec<String> {
    let mut arguments = vec![
        "--socket".to_string(),
        socket_path.to_string(),
        "--port".to_string(),
        port.to_string(),
        "--".to_string(),
    ];
    arguments.extend(inner_tokens.iter().cloned());
    arguments
}

fn normalize_path(path: &str) -> String {
    let trimmed = path.trim();
    if let Some(rest) = trimmed.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join(rest)
                .to_string_lossy()
                .into_owned();
        }
    }
    trimmed.to_string()
}

fn login_shell_environment() -> BTreeMap<String, String> {
    let shell = std::env::var("SHELL")
        .ok()
        .filter(|shell| std::path::Path::new(shell).is_file())
        .unwrap_or_else(|| "/bin/zsh".to_string());
    let output = Command::new(shell)
        .args(["-l", "-c", "printenv -0"])
        .output();
    let Ok(output) = output else {
        return BTreeMap::new();
    };
    if !output.status.success() {
        return BTreeMap::new();
    }
    output
        .stdout
        .split(|byte| *byte == 0)
        .filter_map(|entry| {
            let text = String::from_utf8(entry.to_vec()).ok()?;
            let (key, value) = text.split_once('=')?;
            Some((key.to_string(), value.to_string()))
        })
        .collect()
}

fn split_words(value: &str) -> Vec<String> {
    value.split_whitespace().map(str::to_string).collect()
}

fn shell_escape(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_plan_uses_bridge_when_available() {
        let payload = serde_json::json!({
            "config": {
                "id": "00000000-0000-4000-8000-000000000001",
                "name": "Demo App",
                "command": "npm run dev",
                "arguments": "--host 127.0.0.1",
                "workingDirectory": "~/demo",
                "environment": [{"key": "FOO", "value": "bar"}],
                "portFlag": "--port"
            },
            "context": {
                "bridgePath": "/tmp/rack-bridge"
            }
        });

        let plan = launch_plan_command(&payload);
        let payload = &plan["payload"];
        assert_eq!(payload["subdomain"], "demo-app");
        assert_eq!(payload["useBridge"], true);
        assert_eq!(payload["executable"], "/tmp/rack-bridge");
        assert_eq!(payload["arguments"][0], "--socket");
        assert!(payload["arguments"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("--port")));
        assert_eq!(payload["environment"]["FOO"], "bar");
    }
}
