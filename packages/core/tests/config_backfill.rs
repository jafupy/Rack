use rack_core::config::{backfill_str, BackfillError};

#[test]
fn backfills_missing_top_level_fields() {
    let config = backfill_str(
        r#"# RACK:V1

terminal = "Alacritty"
"#,
    )
    .unwrap();

    assert_eq!(config.schema_version, 1);
    assert!(!config.use_standard_ports);
    assert_eq!(config.terminal, "Alacritty");
    assert!(config.services.is_empty());
}

#[test]
fn backfills_only_safe_service_fields() {
    let config = backfill_str(
        r#"# RACK:V1

use_standard_ports = true
terminal = "Ghostty"

[[services]]
id = "service-1"
name = "API"
run = "cargo run"
"#,
    )
    .unwrap();

    let service = &config.services[0];
    assert_eq!(service.id, "service-1");
    assert_eq!(service.name, "API");
    assert_eq!(service.host, "api");
    assert_eq!(service.run, "cargo run");
    assert_eq!(service.working_dir, "~");
    assert!(!service.auto_start);
}

#[test]
fn rejects_services_missing_required_fields() {
    let error = backfill_str(
        r#"# RACK:V1

use_standard_ports = true
terminal = "Ghostty"

[[services]]
name = "API"
run = "cargo run"
"#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        BackfillError::MissingServiceField {
            index: 0,
            field: "id"
        }
    ));
}
