use super::models::{new_id, PersistedConfiguration, ServerConfiguration};
use serde_json::Value;
use std::path::PathBuf;

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

pub(crate) fn add_server_config_command(payload: &Value) -> Result<PersistedConfiguration, String> {
    let server = if payload.is_null() {
        ServerConfiguration::default()
    } else {
        serde_json::from_value(payload.clone()).map_err(|error| error.to_string())?
    };

    let mut configuration = load_server_config();
    configuration.servers.push(server);
    save_server_config(&configuration)?;
    Ok(configuration)
}

pub(crate) fn duplicate_server_config_command(
    payload: &Value,
) -> Result<PersistedConfiguration, String> {
    let id = payload
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing server id".to_string())?;

    let mut configuration = load_server_config();
    let mut copy = configuration
        .servers
        .iter()
        .find(|server| server.id == id)
        .cloned()
        .ok_or_else(|| format!("server not found: {id}"))?;
    copy.id = new_id();
    copy.name.push_str(" Copy");
    configuration.servers.push(copy);
    save_server_config(&configuration)?;
    Ok(configuration)
}

pub(crate) fn delete_server_config_command(
    payload: &Value,
) -> Result<PersistedConfiguration, String> {
    let ids = payload
        .get("ids")
        .and_then(Value::as_array)
        .ok_or_else(|| "missing server ids".to_string())?;
    let ids: std::collections::HashSet<&str> = ids.iter().filter_map(Value::as_str).collect();

    let mut configuration = load_server_config();
    configuration
        .servers
        .retain(|server| !ids.contains(server.id.as_str()));
    save_server_config(&configuration)?;
    Ok(configuration)
}

pub(super) fn save_server_config(configuration: &PersistedConfiguration) -> Result<(), String> {
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
