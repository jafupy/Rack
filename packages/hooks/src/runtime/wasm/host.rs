use wasmtime::{Caller, Config, Engine, Instance, Linker, Module, Store};

use super::RuntimeError;

pub fn engine() -> Engine {
    let mut config = Config::new();
    config.consume_fuel(true);
    Engine::new(&config).expect("create hook wasm engine")
}

pub fn instantiate(
    engine: &Engine,
    store: &mut Store<()>,
    module: &Module,
) -> Result<Instance, RuntimeError> {
    let mut linker = Linker::new(engine);
    linker.func_wrap(
        "rack",
        "log",
        |mut caller: Caller<'_, ()>, ptr: i32, len: i32| {
            let Some(bytes) = log_bytes(&mut caller, ptr, len) else {
                return;
            };
            eprintln!("{}", String::from_utf8_lossy(bytes));
        },
    )?;
    linker
        .instantiate(store, module)
        .map_err(RuntimeError::from)
}

fn log_bytes<'a>(caller: &'a mut Caller<'_, ()>, ptr: i32, len: i32) -> Option<&'a [u8]> {
    let memory = caller
        .get_export("memory")
        .and_then(|export| export.into_memory())?;
    let start = usize::try_from(ptr).ok()?;
    let len = usize::try_from(len).ok()?;
    let end = start.checked_add(len)?;
    memory.data(caller).get(start..end)
}
