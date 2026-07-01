mod error;
mod exports;
mod host;
mod memory;

use std::collections::HashMap;

use wasmtime::{Engine, Module, Store};

use crate::{CronEvent, HookEndpoint, HookRequest, HookResponse};

use self::{
    exports::validate_module_exports,
    host::{engine, instantiate},
    memory::{memory, read_memory, unpack_ptr_len, write_memory},
};
use super::{load_metadata, metadata, WasmHookEndpoint};

pub use error::RuntimeError;

const HOOK_FUEL: u64 = 10_000_000;

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
            engine: engine(),
            modules: HashMap::new(),
        }
    }

    pub fn load_module(&mut self, wasm: &[u8]) -> Result<Vec<HookEndpoint>, RuntimeError> {
        let metadata = load_metadata(wasm)?;
        let module = Module::from_binary(&self.engine, wasm)?;
        validate_module_exports(&module, &metadata.hooks)?;

        let mut endpoints = Vec::new();
        for hook in metadata.hooks {
            if let WasmHookEndpoint::Http {
                id,
                method,
                path,
                entry,
            } = hook
            {
                endpoints.push(HookEndpoint::new(&id, method, path));
                self.modules.insert(
                    id,
                    WasmModule {
                        module: module.clone(),
                        entry,
                    },
                );
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
    let engine = engine();
    let module = Module::from_binary(&engine, wasm)?;
    let mut store = Store::new(&engine, ());
    store.set_fuel(HOOK_FUEL).map_err(RuntimeError::from)?;
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
        store.set_fuel(HOOK_FUEL).map_err(RuntimeError::from)?;
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
