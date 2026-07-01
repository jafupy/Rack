use wasmtime::{ExternType, Module};

use super::{metadata::WasmHookEndpoint, RuntimeError};

pub fn validate_module_exports(
    module: &Module,
    hooks: &[WasmHookEndpoint],
) -> Result<(), RuntimeError> {
    require_memory(module)?;
    require_func(module, "rack_alloc")?;
    require_func(module, "rack_dealloc")?;
    for hook in hooks {
        match hook {
            WasmHookEndpoint::Http { entry, .. } | WasmHookEndpoint::Cron { entry, .. } => {
                require_func(module, entry)?;
            }
        }
    }
    Ok(())
}

fn require_memory(module: &Module) -> Result<(), RuntimeError> {
    match module.get_export("memory") {
        Some(ExternType::Memory(_)) => Ok(()),
        Some(_) => Err(RuntimeError::InvalidExport("memory".to_string())),
        None => Err(RuntimeError::MissingMemory),
    }
}

fn require_func(module: &Module, name: &str) -> Result<(), RuntimeError> {
    match module.get_export(name) {
        Some(ExternType::Func(_)) => Ok(()),
        Some(_) => Err(RuntimeError::InvalidExport(name.to_string())),
        None => Err(RuntimeError::MissingExport(name.to_string())),
    }
}
