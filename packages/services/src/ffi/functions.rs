use std::{ffi::CStr, os::raw::c_char};

use crate::{runtime::RackRuntime, RUNTIME};

use super::{
    status::{free_string, string_ptr, RackServicesStatus, RackServicesStatusCode, ABI_VERSION},
    types::{free_snapshot, RackServicesServiceSnapshot, RackServicesSnapshot},
};

#[no_mangle]
pub extern "C" fn rack_services_abi_version() -> u32 {
    ABI_VERSION
}

#[no_mangle]
pub extern "C" fn rack_services_status_size() -> usize {
    std::mem::size_of::<RackServicesStatus>()
}

#[no_mangle]
pub extern "C" fn rack_services_snapshot_size() -> usize {
    std::mem::size_of::<RackServicesSnapshot>()
}

#[no_mangle]
pub extern "C" fn rack_services_service_snapshot_size() -> usize {
    std::mem::size_of::<RackServicesServiceSnapshot>()
}

#[no_mangle]
pub extern "C" fn rack_services_init() -> RackServicesStatus {
    status(|| {
        let runtime = RackRuntime::init()?;
        *RUNTIME
            .lock()
            .map_err(|error| FfiError::runtime(error.to_string()))? = Some(runtime);
        Ok(())
    })
}

#[no_mangle]
pub extern "C" fn rack_services_snapshot(
    out: *mut *mut RackServicesSnapshot,
) -> RackServicesStatus {
    status(|| {
        if out.is_null() {
            return Err(FfiError::invalid_argument(
                "expected non-null snapshot output pointer",
            ));
        }

        with_runtime(|runtime| {
            let snapshot = RackServicesSnapshot::from_native(runtime.snapshot()?);
            unsafe { *out = Box::into_raw(Box::new(snapshot)) };
            Ok(())
        })
        .map_err(FfiError::runtime)
    })
}

#[no_mangle]
pub unsafe extern "C" fn rack_services_snapshot_free(snapshot: *mut RackServicesSnapshot) {
    free_snapshot(snapshot);
}

#[no_mangle]
pub extern "C" fn rack_services_start_service(id: *const c_char) -> RackServicesStatus {
    service_command(id, |runtime, id| runtime.start_service(id))
}

#[no_mangle]
pub extern "C" fn rack_services_stop_service(id: *const c_char) -> RackServicesStatus {
    service_command(id, |runtime, id| runtime.stop_service(id))
}

#[no_mangle]
pub extern "C" fn rack_services_restart_service(id: *const c_char) -> RackServicesStatus {
    service_command(id, |runtime, id| runtime.restart_service(id))
}

#[no_mangle]
pub extern "C" fn rack_services_log(id: *const c_char) -> *mut c_char {
    string_result(|| {
        let id = unsafe { c_string(id) }?;
        with_runtime(|runtime| runtime.log(&id))
    })
}

#[no_mangle]
pub extern "C" fn rack_services_log_path(id: *const c_char) -> *mut c_char {
    string_result(|| {
        let id = unsafe { c_string(id) }?;
        with_runtime(|runtime| runtime.log_path(&id))
    })
}

#[no_mangle]
pub extern "C" fn rack_services_shutdown() -> RackServicesStatus {
    status(|| {
        *RUNTIME.lock().map_err(|error| error.to_string())? = None;
        Ok(())
    })
}

#[no_mangle]
pub unsafe extern "C" fn rack_services_status_free(status: RackServicesStatus) {
    free_string(status.message);
}

#[no_mangle]
pub unsafe extern "C" fn rack_services_string_free(value: *mut c_char) {
    free_string(value);
}

#[no_mangle]
pub extern "C" fn rack_services_snapshot_json() -> *mut c_char {
    string_result(|| with_runtime(|runtime| runtime.snapshot_json()))
}

#[no_mangle]
pub extern "C" fn rack_services_hooks_json() -> *mut c_char {
    string_result(|| with_runtime(|runtime| runtime.hooks_json()))
}

fn service_command(
    id: *const c_char,
    command: impl FnOnce(&RackRuntime, &str) -> Result<(), String>,
) -> RackServicesStatus {
    status(|| {
        let id = unsafe { c_string(id) }.map_err(FfiError::invalid_argument)?;
        with_runtime(|runtime| command(runtime, &id)).map_err(FfiError::runtime)?;
        Ok(())
    })
}

fn with_runtime<T>(action: impl FnOnce(&RackRuntime) -> Result<T, String>) -> Result<T, String> {
    let runtime = RUNTIME.lock().map_err(|error| error.to_string())?;
    let runtime = runtime
        .as_ref()
        .ok_or_else(|| "rack services runtime has not been initialized".to_string())?;
    action(runtime)
}

fn status(action: impl FnOnce() -> Result<(), FfiError>) -> RackServicesStatus {
    match action() {
        Ok(()) => RackServicesStatus::ok(),
        Err(error) => RackServicesStatus::error(error.code, error.message),
    }
}

fn string_result(action: impl FnOnce() -> Result<String, String>) -> *mut c_char {
    match action() {
        Ok(value) => string_ptr(value),
        Err(error) => string_ptr(format!("ERROR:{error}")),
    }
}

unsafe fn c_string(value: *const c_char) -> Result<String, String> {
    if value.is_null() {
        return Err("expected non-null service id".to_string());
    }

    CStr::from_ptr(value)
        .to_str()
        .map(str::to_string)
        .map_err(|error| error.to_string())
}

struct FfiError {
    code: RackServicesStatusCode,
    message: String,
}

impl FfiError {
    fn invalid_argument(message: impl Into<String>) -> Self {
        Self {
            code: RackServicesStatusCode::InvalidArgument,
            message: message.into(),
        }
    }

    fn runtime(message: impl Into<String>) -> Self {
        Self {
            code: RackServicesStatusCode::Runtime,
            message: message.into(),
        }
    }
}

impl From<String> for FfiError {
    fn from(error: String) -> Self {
        Self::runtime(error)
    }
}
