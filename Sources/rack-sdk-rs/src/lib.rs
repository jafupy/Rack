//! Rust SDK for Rack function packages.
//!
//! Rack functions are compiled to `wasm32-wasip1`. Rack sends each invocation as
//! JSON on stdin and expects one response JSON object on stdout. This crate hides
//! that boundary behind small route and cron handler macros.
//!
//! HTTP routes usually take a [`Request`] and return a [`Response`]:
//!
//! ```ignore
//! #[rack::route]
//! fn hello(req: rack::Request) -> rack::Response {
//!     rack::log::info(format!("{} {}", req.method(), req.path()));
//!     rack::response::ok().text("hello")
//! }
//! ```
//!
//! JSON request bodies can be parsed into typed payloads:
//!
//! ```ignore
//! #[rack::payload]
//! struct Update {
//!     title: String,
//!     done: bool,
//! }
//!
//! #[rack::route]
//! fn update(req: rack::Request<Update>) -> rack::Response {
//!     rack::response::ok().json(req.body())
//! }
//! ```
//!
//! Route and cron handlers still declare `-> rack::Response`, but the generated
//! wrapper lets `?` work inside the function body. Bad request payloads become
//! `400` responses; handler errors become `500` responses.

pub use rack_macros::{cron, payload, route};
pub use serde;

mod error;
mod http_response;
mod payload;
mod request;

#[path = "private.rs"]
pub mod __private;
pub mod log;
pub mod response;

pub use error::{Error, Result};
pub use http_response::Response;
pub use payload::Payload;
pub use request::{CronEvent, Request, RequestMeta};

/// Build an absolute path to a file in the function package.
///
/// Paths are resolved from the crate root, where `Cargo.toml` lives. Rack
/// function packages keep `manifest.toml` at that same root, so this points at
/// files relative to the package root:
///
/// ```ignore
/// const DATA_PATH: &str = rack::fs!("./public/data.csv");
/// ```
///
/// The macro expands at compile time using `CARGO_MANIFEST_DIR`.
#[macro_export]
macro_rules! fs {
    ($path:literal) => {
        concat!(env!("CARGO_MANIFEST_DIR"), "/", $path)
    };
}
