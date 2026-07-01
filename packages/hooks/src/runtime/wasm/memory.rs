use wasmtime::{Instance, Memory, Store};

use super::RuntimeError;

pub fn memory(store: &mut Store<()>, instance: &Instance) -> Result<Memory, RuntimeError> {
    instance
        .get_memory(store, "memory")
        .ok_or(RuntimeError::MissingMemory)
}

pub fn write_memory(
    store: &mut Store<()>,
    memory: &Memory,
    ptr: i32,
    data: &[u8],
) -> Result<(), RuntimeError> {
    let start = usize::try_from(ptr).map_err(|_| RuntimeError::MemoryBounds)?;
    let end = start
        .checked_add(data.len())
        .ok_or(RuntimeError::MemoryBounds)?;
    memory
        .data_mut(store)
        .get_mut(start..end)
        .ok_or(RuntimeError::MemoryBounds)?
        .copy_from_slice(data);
    Ok(())
}

pub fn read_memory(
    store: &mut Store<()>,
    memory: &Memory,
    ptr: i32,
    len: i32,
) -> Result<Vec<u8>, RuntimeError> {
    let start = usize::try_from(ptr).map_err(|_| RuntimeError::MemoryBounds)?;
    let len = usize::try_from(len).map_err(|_| RuntimeError::MemoryBounds)?;
    let data = memory.data(store);
    let end = start.checked_add(len).ok_or(RuntimeError::MemoryBounds)?;
    Ok(data
        .get(start..end)
        .ok_or(RuntimeError::MemoryBounds)?
        .to_vec())
}

pub fn unpack_ptr_len(value: i64) -> Result<(i32, i32), RuntimeError> {
    let value = u64::try_from(value).map_err(|_| RuntimeError::InvalidPointer)?;
    let ptr = i32::try_from(value >> 32).map_err(|_| RuntimeError::InvalidPointer)?;
    let len = i32::try_from(value & 0xffff_ffff).map_err(|_| RuntimeError::InvalidPointer)?;
    Ok((ptr, len))
}
