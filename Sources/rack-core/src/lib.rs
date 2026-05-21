use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

type EventCallback = extern "C" fn(*const c_char, *mut c_void);

struct CoreState {
    started_at_ms: u128,
    callback: Option<EventCallback>,
    callback_context: usize,
}

static STATE: OnceLock<Mutex<Option<CoreState>>> = OnceLock::new();

fn state() -> &'static Mutex<Option<CoreState>> {
    STATE.get_or_init(|| Mutex::new(None))
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn c_string(value: String) -> *mut c_char {
    CString::new(value)
        .unwrap_or_else(|_| CString::new(r#"{"type":"error","message":"invalid string"}"#).unwrap())
        .into_raw()
}

fn emit(callback: EventCallback, context: usize, json: &str) {
    if let Ok(message) = CString::new(json) {
        callback(message.as_ptr(), context as *mut c_void);
    }
}

#[no_mangle]
pub extern "C" fn rack_core_start(
    config_json: *const c_char,
    callback: Option<EventCallback>,
    callback_context: *mut c_void,
) -> c_int {
    let config = unsafe {
        if config_json.is_null() {
            "{}"
        } else {
            CStr::from_ptr(config_json).to_str().unwrap_or("{}")
        }
    };

    let mut guard = state().lock().unwrap();
    *guard = Some(CoreState {
        started_at_ms: now_ms(),
        callback,
        callback_context: callback_context as usize,
    });

    if let Some(callback) = callback {
        emit(
            callback,
            callback_context as usize,
            &format!(
                r#"{{"type":"core.started","payload":{{"config":{},"backend":"rust"}}}}"#,
                config
            ),
        );
    }

    0
}

#[no_mangle]
pub extern "C" fn rack_core_stop() {
    let mut guard = state().lock().unwrap();
    let previous = guard.take();
    drop(guard);

    if let Some(core) = previous {
        if let Some(callback) = core.callback {
            emit(
                callback,
                core.callback_context,
                r#"{"type":"core.stopped","payload":{"backend":"rust"}}"#,
            );
        }
    }
}

#[no_mangle]
pub extern "C" fn rack_core_command(command_json: *const c_char) -> *mut c_char {
    let command = unsafe {
        if command_json.is_null() {
            ""
        } else {
            CStr::from_ptr(command_json).to_str().unwrap_or("")
        }
    };

    let guard = state().lock().unwrap();
    let Some(core) = guard.as_ref() else {
        return c_string(r#"{"type":"error","message":"rack core is not running"}"#.to_string());
    };

    if command.contains(r#""type":"state.snapshot""#)
        || command.contains(r#""type": "state.snapshot""#)
    {
        return c_string(format!(
            r#"{{"type":"state.snapshot","payload":{{"backend":"rust","started_at_ms":{},"servers":[],"functions":[]}}}}"#,
            core.started_at_ms
        ));
    }

    c_string(format!(
        r#"{{"type":"ack","payload":{{"backend":"rust","command":{}}}}}"#,
        if command.is_empty() { "null" } else { command }
    ))
}

#[no_mangle]
pub extern "C" fn rack_core_free_string(value: *mut c_char) {
    if value.is_null() {
        return;
    }

    unsafe {
        drop(CString::from_raw(value));
    }
}
