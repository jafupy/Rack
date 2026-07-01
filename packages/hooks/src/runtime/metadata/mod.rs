mod section;
mod types;

use thiserror::Error;

use super::METADATA_SECTION;
use section::custom_section;

pub use types::{HookModuleMetadata, WasmHookEndpoint};

#[derive(Debug, Error)]
pub enum MetadataError {
    #[error("missing `{METADATA_SECTION}` wasm custom section")]
    Missing,

    #[error("invalid wasm header")]
    InvalidHeader,

    #[error("invalid wasm section")]
    InvalidSection,

    #[error("invalid hook metadata: {0}")]
    InvalidJson(#[from] serde_json::Error),
}

pub fn load_metadata(wasm: &[u8]) -> Result<HookModuleMetadata, MetadataError> {
    let section = custom_section(wasm, METADATA_SECTION)?.ok_or(MetadataError::Missing)?;
    parse_metadata(section)
}

fn parse_metadata(section: &[u8]) -> Result<HookModuleMetadata, MetadataError> {
    if let Ok(metadata) = serde_json::from_slice(section) {
        return Ok(metadata);
    }

    let hooks = section
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.iter().all(u8::is_ascii_whitespace))
        .map(serde_json::from_slice)
        .collect::<Result<_, _>>()?;
    Ok(HookModuleMetadata { hooks })
}
