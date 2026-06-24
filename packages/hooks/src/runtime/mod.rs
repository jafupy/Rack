mod metadata;
mod wasm;

pub use metadata::{load_metadata, HookModuleMetadata, WasmHookEndpoint};
pub use wasm::{HookRuntime, RuntimeError};

pub const METADATA_SECTION: &str = "rack.hooks";
