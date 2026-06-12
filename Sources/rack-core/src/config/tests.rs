use super::ipc::handle_ipc_message;
use super::models::route_info_command;
use super::storage::{
    add_server_config_command, delete_server_config_command, duplicate_server_config_command,
    load_server_config,
};
use std::time::{SystemTime, UNIX_EPOCH};

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

#[test]
fn server_config_commands_mutate_persisted_state_in_core() {
    let _guard = crate::test_support::env_lock();
    let previous_home = std::env::var_os("HOME");
    let home = temp_home("rack-core-server-config-test");
    std::fs::create_dir_all(&home).unwrap();
    std::env::set_var("HOME", &home);

    let configuration = add_server_config_command(&serde_json::Value::Null).unwrap();
    assert_eq!(configuration.servers.len(), 1);
    assert_eq!(configuration.servers[0].name, "New Server");

    let id = configuration.servers[0].id.clone();
    let duplicated = duplicate_server_config_command(&serde_json::json!({ "id": id })).unwrap();
    assert_eq!(duplicated.servers.len(), 2);
    assert_eq!(duplicated.servers[1].name, "New Server Copy");

    let deleted = delete_server_config_command(&serde_json::json!({
        "ids": [duplicated.servers[0].id]
    }))
    .unwrap();
    assert_eq!(deleted.servers.len(), 1);
    assert_eq!(load_server_config().servers.len(), 1);

    restore_home(previous_home);
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn route_info_command_uses_core_route_rules() {
    let response = route_info_command(&serde_json::json!({
        "config": {
            "name": "Demo App",
            "customDomain": "",
        },
        "context": {
            "boundPort": 1355,
            "standardPortsEnabled": false,
        }
    }))
    .unwrap();

    assert_eq!(response.route_subdomain, "demo-app");
    assert_eq!(response.local_url, "http://demo-app.localhost:1355");
}

fn temp_home(prefix: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "{prefix}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn restore_home(previous_home: Option<std::ffi::OsString>) {
    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }
}
