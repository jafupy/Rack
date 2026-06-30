use rack_hooks::{
    load_metadata, CronEvent, HookRegistry, HookRequest, WasmHookEndpoint, METADATA_SECTION,
};

#[test]
fn reads_hook_metadata_from_wasm_custom_section() {
    let wasm = test_wasm();

    let metadata = load_metadata(&wasm).unwrap();

    assert_eq!(
        metadata.hooks[0],
        WasmHookEndpoint::Http {
            id: "hello".into(),
            method: "GET".into(),
            path: "/hello".into(),
            entry: "hello".into(),
        }
    );
}

#[test]
fn reads_ndjson_hook_metadata_from_wasm_custom_section() {
    let metadata = load_metadata(&with_metadata(
        test_module(),
        br#"{"kind":"http","id":"hello","method":"GET","path":"/hello","entry":"hello"}
{"kind":"cron","id":"tick","schedule":"every minute","entry":"tick"}
"#,
    ))
    .unwrap();

    assert_eq!(metadata.hooks.len(), 2);
    assert_eq!(
        metadata.hooks[1],
        WasmHookEndpoint::Cron {
            id: "tick".into(),
            schedule: "every minute".into(),
            entry: "tick".into(),
        }
    );
}

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
        .register_wasm(&test_wasm_with_response(
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
    registry.register_wasm(&test_wasm()).unwrap();

    let response =
        rack_hooks::dispatch(&registry, &HookRequest::new("GET", "/hello", "rack.local"));

    assert_eq!(response.status, 201);
    assert_eq!(
        response.headers,
        [("content-type".into(), "text/plain".into())]
    );
    assert_eq!(response.body, b"ok");
}

fn test_wasm() -> Vec<u8> {
    test_wasm_with_response(
        r#"{"status":201,"headers":[["content-type","text/plain"]],"body":[111,107]}"#,
    )
}

fn test_wasm_with_response(response: &str) -> Vec<u8> {
    let ptr = 1024u64;
    let len = response.len() as u64;
    let response_data = wat_bytes(response.as_bytes());
    let packed = (ptr << 32) | len;
    let wat = format!(
        r#"
        (module
          (memory (export "memory") 1)
          (global $heap (mut i32) (i32.const 2048))
          (func (export "rack_alloc") (param $len i32) (result i32)
            (local $ptr i32)
            global.get $heap
            local.set $ptr
            global.get $heap
            local.get $len
            i32.add
            global.set $heap
            local.get $ptr)
          (func (export "rack_dealloc") (param i32) (param i32))
          (func (export "hello") (param i32) (param i32) (result i64)
            i64.const {packed})
          (data (i32.const 1024) "{response_data}")
        )
        "#,
    );

    with_metadata(
        wat::parse_str(wat).unwrap(),
        br#"{"hooks":[{"id":"hello","method":"GET","path":"/hello","entry":"hello"}]}"#,
    )
}

fn cron_wasm() -> Vec<u8> {
    wat::parse_str(r#"(module (func (export "tick")))"#).unwrap()
}

fn cron_event_wasm() -> Vec<u8> {
    wat::parse_str(
        r#"
        (module
          (memory (export "memory") 1)
          (global $heap (mut i32) (i32.const 2048))
          (func (export "rack_alloc") (param $len i32) (result i32)
            (local $ptr i32)
            global.get $heap
            local.set $ptr
            global.get $heap
            local.get $len
            i32.add
            global.set $heap
            local.get $ptr)
          (func (export "rack_dealloc") (param i32) (param i32))
          (func (export "tick") (param $ptr i32) (param $len i32)
            local.get $ptr
            i32.load8_u offset=84
            i32.const 52
            i32.ne
            if unreachable end
            local.get $ptr
            i32.load8_u offset=85
            i32.const 50
            i32.ne
            if unreachable end))
        "#,
    )
    .unwrap()
}

fn test_module() -> Vec<u8> {
    wat::parse_str("(module)").unwrap()
}

fn wat_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("\\{byte:02x}")).collect()
}

fn with_metadata(mut wasm: Vec<u8>, metadata: &[u8]) -> Vec<u8> {
    let mut payload = Vec::new();
    write_leb(METADATA_SECTION.len() as u32, &mut payload);
    payload.extend_from_slice(METADATA_SECTION.as_bytes());
    payload.extend_from_slice(metadata);

    let mut section = vec![0];
    write_leb(payload.len() as u32, &mut section);
    section.extend(payload);

    wasm.splice(8..8, section);
    wasm
}

fn write_leb(mut value: u32, out: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
}
