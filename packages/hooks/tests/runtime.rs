use rack_hooks::{load_metadata, HookRegistry, HookRequest, WasmHookEndpoint, METADATA_SECTION};

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
    let response = r#"{"status":201,"headers":[["content-type","text/plain"]],"body":[111,107]}"#;
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
