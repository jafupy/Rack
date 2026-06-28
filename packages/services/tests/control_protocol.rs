use std::collections::HashMap;

use rack_core::config::Service as ServiceConfig;
use rack_services::{
    control::{Command, Request, Response},
    registry::{ServiceState, ServiceView},
    snapshot::{snapshot_service, StateSnapshot},
};

#[test]
fn serializes_commands_as_snake_case() {
    let request = Request {
        command: Command::Restart,
        id: Some("web".to_string()),
        service: None,
    };

    let json = serde_json::to_string(&request).unwrap();

    assert_eq!(json, r#"{"command":"restart","id":"web"}"#);
}

#[test]
fn builds_error_response() {
    let response = Response::error("nope");

    assert!(!response.ok);
    assert_eq!(response.error.as_deref(), Some("nope"));
    assert!(response.snapshot.is_none());
    assert!(response.log.is_none());
}

#[test]
fn snapshots_failed_services_with_reason() {
    let snapshot = snapshot_service(
        ServiceView {
            id: "api".to_string(),
            name: "API".to_string(),
            host: "api".to_string(),
            state: ServiceState::Failed {
                pid: 10,
                pgid: 10,
                reason: "readiness timeout".to_string(),
            },
        },
        &HashMap::from([("api".to_string(), service("api", "api"))]),
    );

    assert_eq!(
        snapshot.state,
        StateSnapshot::Failed {
            pid: 10,
            pgid: 10,
            reason: "readiness timeout".to_string()
        }
    );
}

fn service(id: &str, host: &str) -> ServiceConfig {
    ServiceConfig {
        id: id.to_string(),
        name: id.to_uppercase(),
        host: host.to_string(),
        run: "echo hi".to_string(),
        working_dir: "~".to_string(),
        auto_start: false,
    }
}
