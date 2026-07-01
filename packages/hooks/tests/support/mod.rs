#![allow(dead_code)]

use rack_hooks::METADATA_SECTION;

pub fn http_wasm() -> Vec<u8> {
    http_wasm_with_response(
        r#"{"status":201,"headers":[["content-type","text/plain"]],"body":[111,107]}"#,
    )
}

pub fn http_wasm_with_response(response: &str) -> Vec<u8> {
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

pub fn cron_wasm() -> Vec<u8> {
    wat::parse_str(r#"(module (func (export "tick")))"#).unwrap()
}

pub fn cron_event_wasm() -> Vec<u8> {
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

pub fn empty_module() -> Vec<u8> {
    wat::parse_str("(module)").unwrap()
}

pub fn with_metadata(mut wasm: Vec<u8>, metadata: &[u8]) -> Vec<u8> {
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

fn wat_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("\\{byte:02x}")).collect()
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
