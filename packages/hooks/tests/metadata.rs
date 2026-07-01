mod support;

use rack_hooks::{load_metadata, WasmHookEndpoint};
use support::{empty_module, http_wasm, with_metadata};

#[test]
fn reads_hook_metadata_from_wasm_custom_section() {
    let metadata = load_metadata(&http_wasm()).unwrap();

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
        empty_module(),
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
