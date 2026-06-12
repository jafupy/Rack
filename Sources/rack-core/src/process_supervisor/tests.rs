use super::supervisor_command;

#[test]
fn supervisor_starts_process_and_returns_pid() {
    let _guard = crate::test_support::env_lock();
    let previous_home = std::env::var_os("HOME");
    let home = std::env::temp_dir().join(format!(
        "rack-core-supervisor-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&home).unwrap();
    std::env::set_var("HOME", &home);

    let payload = serde_json::json!({
        "config": {
            "id": "00000000-0000-4000-8000-000000000002",
            "name": "Echo App",
            "command": "/bin/echo",
            "arguments": "hello",
            "workingDirectory": "",
            "environment": []
        },
        "context": {}
    });

    let response = supervisor_command("server.start", &payload, None, 0).unwrap();
    assert_eq!(response["type"], "server.started");
    assert!(response["payload"]["pid"].as_u64().unwrap() > 0);

    match previous_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }
    let _ = std::fs::remove_dir_all(home);
}
