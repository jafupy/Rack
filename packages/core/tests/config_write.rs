use std::{fs, path::PathBuf};

use rack_core::config::{
    add_service, remove_service, replace_service, save_at, set_terminal, Config, Service,
    WriteError,
};

#[test]
fn writes_source_config_with_schema_header() {
    let dir = test_dir("writes_source_config_with_schema_header");
    let path = dir.join("rack/config.toml");

    save_at(path.clone(), &config()).unwrap();

    let output = fs::read_to_string(path).unwrap();
    assert!(output.starts_with("# RACK:V1\n\n"));
    assert!(output.contains("[[services]]"));
    assert!(output.contains("host = \"api\""));
}

#[test]
fn updates_terminal_in_memory() {
    let mut config = config();

    set_terminal(&mut config, "Terminal").unwrap();

    assert_eq!(config.terminal, "Terminal");
}

#[test]
fn rejects_blank_terminal_without_mutating_config() {
    let mut config = config();
    let before = config.clone();

    let error = set_terminal(&mut config, "   ").unwrap_err();

    assert!(matches!(error, WriteError::BlankTerminal));
    assert_eq!(config, before);
}

#[test]
fn adds_replaces_and_removes_services_in_memory() {
    let mut config = config();
    let worker = Service {
        id: "worker".to_string(),
        name: "Worker".to_string(),
        host: "worker".to_string(),
        run: "bun worker".to_string(),
        working_dir: "~/worker".to_string(),
        auto_start: true,
    };

    add_service(&mut config, worker.clone()).unwrap();
    assert_eq!(config.services.len(), 2);

    let edited = Service {
        name: "Background Worker".to_string(),
        ..worker.clone()
    };
    replace_service(&mut config, "worker", edited).unwrap();
    assert_eq!(config.services[1].name, "Background Worker");

    let removed = remove_service(&mut config, "worker").unwrap();
    assert_eq!(removed.id, "worker");
    assert_eq!(config.services.len(), 1);
}

#[test]
fn rejects_duplicate_service_add_without_mutating_config() {
    let mut config = config();
    let before = config.clone();
    let duplicate = config.services[0].clone();

    let error = add_service(&mut config, duplicate).unwrap_err();

    assert!(matches!(error, WriteError::ServiceAlreadyExists(id) if id == "api"));
    assert_eq!(config, before);
}

#[test]
fn rejects_invalid_add_without_mutating_config() {
    let mut config = config();
    let before = config.clone();
    let mut duplicate_host = Service {
        id: "worker".to_string(),
        name: "Worker".to_string(),
        host: "api".to_string(),
        run: "bun worker".to_string(),
        working_dir: "~/worker".to_string(),
        auto_start: false,
    };

    let error = add_service(&mut config, duplicate_host.clone()).unwrap_err();
    assert!(matches!(error, WriteError::Validate(_)));
    assert_eq!(config, before);

    duplicate_host.host = "API".to_string();
    let error = add_service(&mut config, duplicate_host).unwrap_err();
    assert!(matches!(error, WriteError::Validate(_)));
    assert_eq!(config, before);
}

#[test]
fn rejects_mismatched_edit_id_without_mutating_config() {
    let mut config = config();
    let before = config.clone();
    let mut edited = config.services[0].clone();
    edited.id = "worker".to_string();

    let error = replace_service(&mut config, "api", edited).unwrap_err();

    assert!(
        matches!(error, WriteError::ServiceIdMismatch { target_id, service_id } if target_id == "api" && service_id == "worker")
    );
    assert_eq!(config, before);
}

#[test]
fn rejects_invalid_edit_without_mutating_config() {
    let mut config = config();
    let before = config.clone();
    let mut edited = config.services[0].clone();
    edited.host = "API".to_string();

    let error = replace_service(&mut config, "api", edited).unwrap_err();

    assert!(matches!(error, WriteError::Validate(_)));
    assert_eq!(config, before);
}

#[test]
fn rejects_invalid_config_before_writing() {
    let dir = test_dir("rejects_invalid_config_before_writing");
    let path = dir.join("rack/config.toml");
    let mut config = config();
    config.services[0].host = "API".to_string();

    let error = save_at(path.clone(), &config).unwrap_err();

    assert!(matches!(error, WriteError::Validate(_)));
    assert!(!path.exists());
}

fn config() -> Config {
    Config {
        schema_version: 1,
        use_standard_ports: false,
        terminal: "Ghostty".to_string(),
        services: vec![Service {
            id: "api".to_string(),
            name: "API".to_string(),
            host: "api".to_string(),
            run: "bun dev".to_string(),
            working_dir: "~/api".to_string(),
            auto_start: false,
        }],
    }
}

fn test_dir(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("rack-core-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    path
}
