use super::MetadataError;

pub fn custom_section<'a>(wasm: &'a [u8], wanted: &str) -> Result<Option<&'a [u8]>, MetadataError> {
    if wasm.get(..8) != Some(b"\0asm\x01\0\0\0") {
        return Err(MetadataError::InvalidHeader);
    }

    let mut offset = 8;
    while offset < wasm.len() {
        let section_id = wasm[offset];
        offset += 1;
        let section_len = read_leb(wasm, &mut offset)? as usize;
        let section_end = checked_end(offset, section_len, wasm.len())?;

        if section_id == 0 {
            let mut cursor = offset;
            let name_len = read_leb(wasm, &mut cursor)? as usize;
            let name_end = checked_end(cursor, name_len, section_end)?;
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

fn checked_end(start: usize, len: usize, limit: usize) -> Result<usize, MetadataError> {
    start
        .checked_add(len)
        .filter(|end| *end <= limit)
        .ok_or(MetadataError::InvalidSection)
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
