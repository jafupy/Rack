mod command;
mod config;
mod dev_commands;
mod functions;
mod ipc;
mod process;
mod process_readiness;
mod process_supervisor;
mod project;
mod proxy;
mod routes;
mod schedule;

#[cfg(test)]
mod test_support {
    use std::sync::{Mutex, MutexGuard, OnceLock};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    pub(crate) fn env_lock() -> MutexGuard<'static, ()> {
        ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }
}

use functions::start_scheduler;
use ipc::start_ipc_server;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) type EventCallback = extern "C" fn(*const c_char, *mut c_void);

struct CoreState {
    started_at_ms: u128,
    callback: Option<EventCallback>,
    callback_context: usize,
    scheduler_stop: Arc<AtomicBool>,
    ipc_stop: Arc<AtomicBool>,
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

pub(crate) fn emit(callback: EventCallback, context: usize, json: &str) {
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

    let scheduler_stop = Arc::new(AtomicBool::new(false));
    start_scheduler(scheduler_stop.clone(), callback, callback_context as usize);
    let ipc_stop = Arc::new(AtomicBool::new(false));
    start_ipc_server(ipc_stop.clone(), callback, callback_context as usize);

    let mut guard = state().lock().unwrap();
    *guard = Some(CoreState {
        started_at_ms: now_ms(),
        callback,
        callback_context: callback_context as usize,
        scheduler_stop,
        ipc_stop,
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
        core.scheduler_stop.store(true, Ordering::Relaxed);
        core.ipc_stop.store(true, Ordering::Relaxed);
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

    c_string(command::handle_command(command))
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

pub(crate) fn callback_info() -> (Option<EventCallback>, usize) {
    let guard = state().lock().unwrap();
    let callback = guard.as_ref().and_then(|core| core.callback);
    let callback_context = guard
        .as_ref()
        .map(|core| core.callback_context)
        .unwrap_or_default();
    (callback, callback_context)
}

pub(crate) fn started_at_ms() -> Option<u128> {
    state()
        .lock()
        .unwrap()
        .as_ref()
        .map(|core| core.started_at_ms)
}
