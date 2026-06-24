use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::METADATA_SECTION;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookModuleMetadata {
    pub hooks: Vec<WasmHookEndpoint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmHookEndpoint {
    pub id: String,
    pub method: String,
    pub path: String,
    pub entry: String,
}

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
    Ok(serde_json::from_slice(section)?)
}

fn custom_section<'a>(wasm: &'a [u8], wanted: &str) -> Result<Option<&'a [u8]>, MetadataError> {
    if wasm.get(..8) != Some(b"\0asm\x01\0\0\0") {
        return Err(MetadataError::InvalidHeader);
    }

    let mut offset = 8;
    while offset < wasm.len() {
        let section_id = wasm[offset];
        offset += 1;
        let section_len = read_leb(wasm, &mut offset)? as usize;
        let section_end = offset
            .checked_add(section_len)
            .filter(|end| *end <= wasm.len())
            .ok_or(MetadataError::InvalidSection)?;

        if section_id == 0 {
            let mut cursor = offset;
            let name_len = read_leb(wasm, &mut cursor)? as usize;
            let name_end = cursor
                .checked_add(name_len)
                .filter(|end| *end <= section_end)
                .ok_or(MetadataError::InvalidSection)?;
            let name = std::str::from_utf8(&wasm[cursor..name_end])
                .map_err(|_| MetadataError::InvalidSection)?;
            if name == wanted {
                return Ok(Some(&wasm[name_end..section_end]));
            }
        }

        offset = section_end;
    }

    Ok(None)
}

fn read_leb(wasm: &[u8], offset: &mut usize) -> Result<u32, MetadataError> {
    let mut result = 0u32;
    let mut shift = 0;

    loop {
        let byte = *wasm.get(*offset).ok_or(MetadataError::InvalidSection)?;
        *offset += 1;
        result |= u32::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
        shift += 7;
        if shift >= 32 {
            return Err(MetadataError::InvalidSection);
        }
    }
}
