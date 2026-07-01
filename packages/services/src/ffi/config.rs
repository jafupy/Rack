use std::os::raw::c_char;

use crate::runtime::RackRuntime;

use super::{
    status::RackServicesStatus,
    support::{c_string, status, string_result, FfiError},
};

#[no_mangle]
pub extern "C" fn rack_services_config_path() -> *mut c_char {
    string_result(RackRuntime::config_path)
}

#[no_mangle]
pub extern "C" fn rack_services_terminal() -> *mut c_char {
    string_result(RackRuntime::terminal)
}

#[no_mangle]
pub extern "C" fn rack_services_set_terminal(terminal: *const c_char) -> RackServicesStatus {
    status(|| {
        let terminal = unsafe { c_string(terminal) }.map_err(FfiError::invalid_argument)?;
        RackRuntime::set_terminal(&terminal).map_err(FfiError::runtime)?;
        Ok(())
    })
}
