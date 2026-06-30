mod abi;
mod cron;
mod http;
mod payload;
mod response;

pub use cron::{Cron, CronEvent};
pub use http::Request;
pub use payload::Payload;
pub use rack_sdk_macro::{cron, payload, route};
pub use response::{Error, IntoResponse, Response, Result};
pub use serde;

#[cfg(target_arch = "wasm32")]
pub fn log(message: impl AsRef<str>) {
    let message = message.as_ref();
    unsafe { rack_log(message.as_ptr() as i32, message.len() as i32) }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn log(message: impl AsRef<str>) {
    eprintln!("{}", message.as_ref());
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "rack")]
extern "C" {
    #[link_name = "log"]
    fn rack_log(ptr: i32, len: i32);
}

#[doc(hidden)]
pub mod __private {
    pub use crate::abi::{read_cron_event, run_http, run_http_empty};
    pub use crate::payload::{from_json as payload_from_json, to_json as payload_to_json};
}
