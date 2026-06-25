mod metadata;
mod wasm;

pub use metadata::{load_metadata, HookModuleMetadata, WasmHookEndpoint};
pub use wasm::{run_cron_wasm, HookRuntime, RuntimeError};

pub const METADATA_SECTION: &str = "rack.hooks";
