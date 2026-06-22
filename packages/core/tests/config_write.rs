use std::{fs, path::PathBuf};

use rack_core::config::{save_at, Config, Service, WriteError};

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
