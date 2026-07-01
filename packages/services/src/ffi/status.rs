use std::{ffi::CString, os::raw::c_char};

pub const ABI_VERSION: u32 = 1;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RackServicesStatusCode {
    Ok = 0,
    InvalidArgument = 1,
    Runtime = 2,
    Internal = 3,
    AlreadyInitialized = 4,
}

#[repr(C)]
#[derive(Debug)]
pub struct RackServicesStatus {
    pub abi_version: u32,
    pub code: RackServicesStatusCode,
    pub message: *mut c_char,
}

impl RackServicesStatus {
    pub fn ok() -> Self {
        Self {
            abi_version: ABI_VERSION,
            code: RackServicesStatusCode::Ok,
            message: std::ptr::null_mut(),
        }
    }

    pub fn error(code: RackServicesStatusCode, message: impl Into<String>) -> Self {
        Self {
            abi_version: ABI_VERSION,
            code,
            message: string_ptr(message.into()),
        }
    }
}

pub fn string_ptr(value: String) -> *mut c_char {
    CString::new(value)
        .unwrap_or_else(|_| CString::new("response contained nul byte").unwrap())
        .into_raw()
}

pub unsafe fn free_string(value: *mut c_char) {
    if !value.is_null() {
        let _ = CString::from_raw(value);
    }
}
