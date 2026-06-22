use std::ffi::{CStr, CString};
use std::os::raw::c_char;

pub fn result(action: impl FnOnce() -> Result<String, String>) -> *mut c_char {
    let output = match action() {
        Ok(value) => value,
        Err(error) => format!("ERROR:{error}"),
    };

    CString::new(output)
        .unwrap_or_else(|_| CString::new("ERROR:response contained nul byte").unwrap())
        .into_raw()
}

pub unsafe fn string(value: *const c_char) -> Result<String, String> {
    if value.is_null() {
        return Err("expected non-null service id".to_string());
    }

    CStr::from_ptr(value)
        .to_str()
        .map(str::to_string)
        .map_err(|error| error.to_string())
}

pub unsafe fn free(value: *mut c_char) {
    if !value.is_null() {
        let _ = CString::from_raw(value);
    }
}
