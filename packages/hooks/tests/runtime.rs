mod support;

use rack_hooks::{CronEvent, HookRegistry, HookRequest};
use support::{cron_event_wasm, cron_wasm, http_wasm, http_wasm_with_response, with_metadata};

#[test]
fn executes_wasm_cron_hook() {
    rack_hooks::run_cron_wasm(&cron_wasm(), "tick").unwrap();
}

#[test]
fn executes_wasm_cron_hook_with_event_payload() {
    let event = CronEvent::new("demo", "tick", "weekdays at 9:30am", 42);

    rack_hooks::run_cron_wasm_with_event(&cron_event_wasm(), "tick", &event).unwrap();
}

#[test]
fn errors_for_missing_wasm_cron_export() {
    assert!(rack_hooks::run_cron_wasm(&cron_wasm(), "missing").is_err());
}

#[test]
fn rejects_wasm_missing_memory_export() {
    let registry = HookRegistry::default();
    let wasm = with_metadata(
        wat::parse_str(r#"(module (func (export "rack_alloc") (param i32) (result i32) i32.const 0) (func (export "rack_dealloc") (param i32) (param i32)) (func (export "hello") (param i32) (param i32) (result i64) i64.const 0))"#).unwrap(),
        br#"{"hooks":[{"id":"hello","method":"GET","path":"/hello","entry":"hello"}]}"#,
    );

    let error = registry.register_wasm(&wasm).unwrap_err().to_string();

    assert_eq!(error, "hook wasm does not export memory");
}

#[test]
fn rejects_wasm_missing_route_export() {
    let registry = HookRegistry::default();
    let wasm = with_metadata(
        wat::parse_str(r#"(module (memory (export "memory") 1) (func (export "rack_alloc") (param i32) (result i32) i32.const 0) (func (export "rack_dealloc") (param i32) (param i32)))"#).unwrap(),
        br#"{"hooks":[{"id":"hello","method":"GET","path":"/hello","entry":"hello"}]}"#,
    );

    let error = registry.register_wasm(&wasm).unwrap_err().to_string();

    assert_eq!(error, "hook wasm does not export `hello`");
}

#[test]
fn rejects_wasm_hook_responses_with_invalid_statuses() {
    let registry = HookRegistry::default();
    registry
        .register_wasm(&http_wasm_with_response(
            r#"{"status":99,"headers":[],"body":[]}"#,
        ))
        .unwrap();

    let response =
        rack_hooks::dispatch(&registry, &HookRequest::new("GET", "/hello", "rack.local"));

    assert_eq!(response.status, 500);
    assert_eq!(
        response.body,
        b"hook failed: invalid HTTP status returned by hook: 99\n"
    );
}

#[test]
fn executes_registered_wasm_hook() {
    let registry = HookRegistry::default();
    registry.register_wasm(&http_wasm()).unwrap();

    let response =
        rack_hooks::dispatch(&registry, &HookRequest::new("GET", "/hello", "rack.local"));

    assert_eq!(response.status, 201);
    assert_eq!(
        response.headers,
        [("content-type".into(), "text/plain".into())]
    );
    assert_eq!(response.body, b"ok");
}
