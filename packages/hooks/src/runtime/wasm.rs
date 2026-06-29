use std::collections::HashMap;

use thiserror::Error;
use wasmtime::{Caller, Engine, Instance, Linker, Memory, Module, Store};

use crate::{CronEvent, HookEndpoint, HookRequest, HookResponse};

use super::{load_metadata, metadata::MetadataError, WasmHookEndpoint};

pub struct HookRuntime {
    engine: Engine,
    modules: HashMap<String, WasmModule>,
}

impl Default for HookRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl HookRuntime {
    pub fn new() -> Self {
        Self {
            engine: Engine::default(),
            modules: HashMap::new(),
        }
    }

    pub fn load_module(&mut self, wasm: &[u8]) -> Result<Vec<HookEndpoint>, RuntimeError> {
        let metadata = load_metadata(wasm)?;
        let module = Module::from_binary(&self.engine, wasm)?;
        let mut endpoints = Vec::new();

        for hook in metadata.hooks {
            match hook {
                WasmHookEndpoint::Http {
                    id,
                    method,
                    path,
                    entry,
                } => {
                    endpoints.push(HookEndpoint::new(&id, method, path));
                    self.modules.insert(
                        id,
                        WasmModule {
                            module: module.clone(),
                            entry,
                        },
                    );
                }
                WasmHookEndpoint::Cron { .. } => {}
            }
        }

        Ok(endpoints)
    }

    pub fn run(
        &self,
        endpoint: &HookEndpoint,
        request: &HookRequest,
    ) -> Result<HookResponse, RuntimeError> {
        let module = self
            .modules
            .get(&endpoint.id)
            .ok_or(RuntimeError::MissingModule)?;
        module.run(&self.engine, request)
    }
}

pub fn run_cron_wasm(wasm: &[u8], entry: &str) -> Result<(), RuntimeError> {
    let event = CronEvent::new("", entry, "", 0);
    run_cron_wasm_with_event(wasm, entry, &event)
}

pub fn run_cron_wasm_with_event(
    wasm: &[u8],
    entry: &str,
    event: &CronEvent,
) -> Result<(), RuntimeError> {
    let engine = Engine::default();
    let module = Module::from_binary(&engine, wasm)?;
    let mut store = Store::new(&engine, ());
    let instance = instantiate(&engine, &mut store, &module)?;

    if let Ok(entry_func) = instance.get_typed_func::<(i32, i32), ()>(&mut store, entry) {
        let memory = memory(&mut store, &instance)?;
        let alloc = instance.get_typed_func::<i32, i32>(&mut store, "rack_alloc")?;
        let dealloc = instance.get_typed_func::<(i32, i32), ()>(&mut store, "rack_dealloc")?;
        let event = serde_json::to_vec(event)?;
        let event_ptr = alloc.call(&mut store, event.len() as i32)?;
        write_memory(&mut store, &memory, event_ptr, &event)?;
        let result = entry_func.call(&mut store, (event_ptr, event.len() as i32));
        dealloc.call(&mut store, (event_ptr, event.len() as i32))?;
        return result.map_err(RuntimeError::from);
    }

    let entry_func = instance.get_typed_func::<(), ()>(&mut store, entry)?;
    entry_func.call(&mut store, ()).map_err(RuntimeError::from)
}

struct WasmModule {
    module: Module,
    entry: String,
}

impl WasmModule {
    fn run(&self, engine: &Engine, request: &HookRequest) -> Result<HookResponse, RuntimeError> {
        let mut store = Store::new(engine, ());
        let instance = instantiate(engine, &mut store, &self.module)?;
        let memory = memory(&mut store, &instance)?;
        let alloc = instance.get_typed_func::<i32, i32>(&mut store, "rack_alloc")?;
        let dealloc = instance.get_typed_func::<(i32, i32), ()>(&mut store, "rack_dealloc")?;
        let entry = instance.get_typed_func::<(i32, i32), i64>(&mut store, &self.entry)?;

        let request = serde_json::to_vec(request)?;
        let request_ptr = alloc.call(&mut store, request.len() as i32)?;
        write_memory(&mut store, &memory, request_ptr, &request)?;

        let packed = entry.call(&mut store, (request_ptr, request.len() as i32))?;
        dealloc.call(&mut store, (request_ptr, request.len() as i32))?;

        let (response_ptr, response_len) = unpack_ptr_len(packed)?;
        let response = read_memory(&mut store, &memory, response_ptr, response_len)?;
        dealloc.call(&mut store, (response_ptr, response_len))?;

        let response: HookResponse = serde_json::from_slice(&response)?;
        response.validate().map_err(RuntimeError::InvalidResponse)?;
        Ok(response)
    }
}

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

    #[error("hook wasm memory access is out of bounds")]
    MemoryBounds,

    #[error("invalid pointer/length returned by hook")]
    InvalidPointer,

    #[error(transparent)]
    InvalidResponse(#[from] crate::InvalidHookResponse),
}

fn instantiate(
    engine: &Engine,
    store: &mut Store<()>,
    module: &Module,
) -> Result<Instance, RuntimeError> {
    let mut linker = Linker::new(engine);
    linker.func_wrap(
        "rack",
        "log",
        |mut caller: Caller<'_, ()>, ptr: i32, len: i32| {
            let Some(memory) = caller
                .get_export("memory")
                .and_then(|export| export.into_memory())
            else {
                return;
            };
            let Ok(start) = usize::try_from(ptr) else {
                return;
            };
            let Ok(len) = usize::try_from(len) else {
                return;
            };
            let data = memory.data(&caller);
            if let Some(bytes) = data.get(start..start + len) {
                eprintln!("{}", String::from_utf8_lossy(bytes));
            }
        },
    )?;
    linker
        .instantiate(store, module)
        .map_err(RuntimeError::from)
}

fn memory(store: &mut Store<()>, instance: &Instance) -> Result<Memory, RuntimeError> {
    instance
        .get_memory(store, "memory")
        .ok_or(RuntimeError::MissingMemory)
}

fn write_memory(
    store: &mut Store<()>,
    memory: &Memory,
    ptr: i32,
    data: &[u8],
) -> Result<(), RuntimeError> {
    let start = usize::try_from(ptr).map_err(|_| RuntimeError::MemoryBounds)?;
    memory
        .data_mut(store)
        .get_mut(start..start + data.len())
        .ok_or(RuntimeError::MemoryBounds)?
        .copy_from_slice(data);
    Ok(())
}

fn read_memory(
    store: &mut Store<()>,
    memory: &Memory,
    ptr: i32,
    len: i32,
) -> Result<Vec<u8>, RuntimeError> {
    let start = usize::try_from(ptr).map_err(|_| RuntimeError::MemoryBounds)?;
    let len = usize::try_from(len).map_err(|_| RuntimeError::MemoryBounds)?;
    let data = memory.data(store);
    Ok(data
        .get(start..start + len)
        .ok_or(RuntimeError::MemoryBounds)?
        .to_vec())
}

fn unpack_ptr_len(value: i64) -> Result<(i32, i32), RuntimeError> {
    let value = u64::try_from(value).map_err(|_| RuntimeError::InvalidPointer)?;
    let ptr = i32::try_from(value >> 32).map_err(|_| RuntimeError::InvalidPointer)?;
    let len = i32::try_from(value & 0xffff_ffff).map_err(|_| RuntimeError::InvalidPointer)?;
    Ok((ptr, len))
}
