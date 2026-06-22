pub mod control;
mod ffi;
pub mod process;
pub mod registry;
mod runtime;
pub mod snapshot;
pub mod supervisor;

use std::{os::raw::c_char, sync::Mutex};

use runtime::RackRuntime;

static RUNTIME: Mutex<Option<RackRuntime>> = Mutex::new(None);

#[no_mangle]
pub extern "C" fn rack_services_init() -> *mut c_char {
    ffi::result(|| {
        let runtime = RackRuntime::init()?;
        *RUNTIME.lock().map_err(|error| error.to_string())? = Some(runtime);
        Ok(String::new())
    })
}

#[no_mangle]
pub extern "C" fn rack_services_snapshot_json() -> *mut c_char {
    with_runtime(|runtime| runtime.snapshot_json())
}

#[no_mangle]
pub extern "C" fn rack_services_start_service(id: *const c_char) -> *mut c_char {
    with_service_id(id, |runtime, id| {
        runtime.start_service(id)?;
        Ok(String::new())
    })
}

#[no_mangle]
pub extern "C" fn rack_services_stop_service(id: *const c_char) -> *mut c_char {
    with_service_id(id, |runtime, id| {
        runtime.stop_service(id)?;
        Ok(String::new())
    })
}

#[no_mangle]
pub extern "C" fn rack_services_restart_service(id: *const c_char) -> *mut c_char {
    with_service_id(id, |runtime, id| {
        runtime.restart_service(id)?;
        Ok(String::new())
    })
}

#[no_mangle]
pub extern "C" fn rack_services_log(id: *const c_char) -> *mut c_char {
    with_service_id(id, |runtime, id| runtime.log(id))
}

#[no_mangle]
pub extern "C" fn rack_services_shutdown() -> *mut c_char {
    ffi::result(|| {
        *RUNTIME.lock().map_err(|error| error.to_string())? = None;
        Ok(String::new())
    })
}

#[no_mangle]
pub unsafe extern "C" fn rack_services_string_free(value: *mut c_char) {
    ffi::free(value);
}

fn with_service_id(
    id: *const c_char,
    action: impl FnOnce(&RackRuntime, &str) -> Result<String, String>,
) -> *mut c_char {
    ffi::result(|| {
        let id = unsafe { ffi::string(id) }?;
        let runtime = RUNTIME.lock().map_err(|error| error.to_string())?;
        let runtime = runtime
            .as_ref()
            .ok_or_else(|| "rack services runtime has not been initialized".to_string())?;
        action(runtime, &id)
    })
}

fn with_runtime(action: impl FnOnce(&RackRuntime) -> Result<String, String>) -> *mut c_char {
    ffi::result(|| {
        let runtime = RUNTIME.lock().map_err(|error| error.to_string())?;
        let runtime = runtime
            .as_ref()
            .ok_or_else(|| "rack services runtime has not been initialized".to_string())?;
        action(runtime)
    })
}
