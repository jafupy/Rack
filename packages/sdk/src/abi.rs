use crate::{Request, Response};

#[no_mangle]
pub extern "C" fn rack_alloc(len: i32) -> i32 {
    let len = usize::try_from(len).expect("negative allocation length");
    let mut bytes = Vec::<u8>::with_capacity(len);
    let ptr = bytes.as_mut_ptr();
    std::mem::forget(bytes);
    ptr as i32
}

#[no_mangle]
pub unsafe extern "C" fn rack_dealloc(ptr: i32, len: i32) {
    let len = usize::try_from(len).expect("negative allocation length");
    drop(Vec::from_raw_parts(ptr as *mut u8, len, len));
}

pub fn run_http(handler: fn(Request) -> Response, req_ptr: i32, req_len: i32) -> i64 {
    let request = read_request(req_ptr, req_len);
    let response = handler(request);
    let bytes = serde_json::to_vec(&response).expect("serialize hook response");
    let (ptr, len) = leak_bytes(bytes);
    pack_ptr_len(ptr, len)
}

fn read_request(ptr: i32, len: i32) -> Request {
    let len = usize::try_from(len).expect("negative request length");
    let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) };
    serde_json::from_slice(bytes).expect("deserialize hook request")
}

fn leak_bytes(bytes: Vec<u8>) -> (i32, i32) {
    let len = bytes.len();
    let boxed = bytes.into_boxed_slice();
    let ptr = Box::into_raw(boxed) as *mut u8;
    (ptr as i32, len as i32)
}

fn pack_ptr_len(ptr: i32, len: i32) -> i64 {
    ((ptr as u64) << 32 | len as u32 as u64) as i64
}
