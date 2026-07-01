use std::{ffi::CStr, os::raw::c_char};

use rack_core::config::Service as ServiceConfig;

use crate::{runtime::RackRuntime, RUNTIME};

use super::status::{string_ptr, RackServicesStatus, RackServicesStatusCode};

pub(super) fn with_runtime<T>(
    action: impl FnOnce(&RackRuntime) -> Result<T, String>,
) -> Result<T, String> {
    let runtime = RUNTIME.lock().map_err(|error| error.to_string())?;
    let runtime = runtime
        .as_ref()
        .ok_or_else(|| "rack services runtime has not been initialized".to_string())?;
    action(runtime)
}

pub(super) fn with_runtime_mut<T>(
    action: impl FnOnce(&mut RackRuntime) -> Result<T, String>,
) -> Result<T, String> {
    let mut runtime = RUNTIME.lock().map_err(|error| error.to_string())?;
    let runtime = runtime
        .as_mut()
        .ok_or_else(|| "rack services runtime has not been initialized".to_string())?;
    action(runtime)
}

pub(super) fn status(action: impl FnOnce() -> Result<(), FfiError>) -> RackServicesStatus {
    match action() {
        Ok(()) => RackServicesStatus::ok(),
        Err(error) => RackServicesStatus::error(error.code, error.message),
    }
}

pub(super) fn string_result(action: impl FnOnce() -> Result<String, String>) -> *mut c_char {
    match action() {
        Ok(value) => string_ptr(value),
        Err(error) => string_ptr(format!("ERROR:{error}")),
    }
}

pub(super) unsafe fn service_config(value: *const c_char) -> Result<ServiceConfig, String> {
    let json = c_string(value)?;
    serde_json::from_str(&json).map_err(|error| error.to_string())
}

pub(super) unsafe fn c_string(value: *const c_char) -> Result<String, String> {
    if value.is_null() {
        return Err("expected non-null string".to_string());
    }

    CStr::from_ptr(value)
        .to_str()
        .map(str::to_string)
        .map_err(|error| error.to_string())
}

pub(super) struct FfiError {
    code: RackServicesStatusCode,
    message: String,
}

impl FfiError {
    pub(super) fn invalid_argument(message: impl Into<String>) -> Self {
        Self {
            code: RackServicesStatusCode::InvalidArgument,
            message: message.into(),
        }
    }

    pub(super) fn runtime(message: impl Into<String>) -> Self {
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
