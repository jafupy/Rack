use rack_services::control::{Command, Request, Response};

#[test]
fn serializes_commands_as_snake_case() {
    let request = Request {
        command: Command::Restart,
        id: Some("web".to_string()),
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
