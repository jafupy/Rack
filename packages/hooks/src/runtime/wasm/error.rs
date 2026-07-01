use thiserror::Error;

use super::super::metadata::MetadataError;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error(transparent)]
    Metadata(#[from] MetadataError),

    #[error(transparent)]
    Wasm(#[from] wasmtime::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("hook module is not loaded")]
    MissingModule,

    #[error("hook wasm does not export memory")]
    MissingMemory,

    #[error("hook wasm does not export `{0}`")]
    MissingExport(String),

    #[error("hook wasm export `{0}` has the wrong type")]
    InvalidExport(String),

    #[error("hook wasm memory access is out of bounds")]
    MemoryBounds,

    #[error("invalid pointer/length returned by hook")]
    InvalidPointer,

    #[error(transparent)]
    InvalidResponse(#[from] crate::InvalidHookResponse),
}
