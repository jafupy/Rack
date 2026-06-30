use crate::{CronEvent, IntoResponse, Payload, Request, Response};

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

pub fn run_http<T, R>(handler: fn(Request<T>) -> R, req_ptr: i32, req_len: i32) -> i64
where
    T: Payload,
    R: IntoResponse,
{
    let response = read_request(req_ptr, req_len)
        .and_then(Request::parse_payload)
        .map(|request| handler(request).into_response())
        .unwrap_or_else(|error| Response::bad_request(error.to_string()));
    write_response(response)
}

pub fn run_http_empty<R: IntoResponse>(handler: fn() -> R, req_ptr: i32, req_len: i32) -> i64 {
    let response = read_request(req_ptr, req_len)
        .map(|_| handler().into_response())
        .unwrap_or_else(|error| Response::bad_request(error.to_string()));
    write_response(response)
}

pub fn read_cron_event(ptr: i32, len: i32) -> CronEvent {
    let len = usize::try_from(len).expect("negative cron event length");
    let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) };
    serde_json::from_slice(bytes).expect("deserialize cron event")
}

fn read_request(ptr: i32, len: i32) -> crate::Result<Request> {
    let len = usize::try_from(len).expect("negative request length");
    let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) };
    serde_json::from_slice(bytes).map_err(crate::Error::from)
}

fn write_response(response: Response) -> i64 {
    let bytes = serde_json::to_vec(&response).expect("serialize hook response");
    let (ptr, len) = leak_bytes(bytes);
    pack_ptr_len(ptr, len)
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
