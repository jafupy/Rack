use std::{fs, path::Path};

use anyhow::{Context, Result};
use rack_hooks::{
    dispatch, load_metadata, run_cron_wasm_with_event, CronEvent, HookRegistry, HookRequest,
};

use super::build;

#[path = "test/artifact.rs"]
mod artifact;
#[path = "test/target.rs"]
mod target;

use artifact::built_wasm_path;
use target::{select_test_target, TestTarget};

pub fn run(path: &str, hook: Option<&str>, route: Option<&str>) -> Result<()> {
    build::run(path)?;
    let wasm_path = built_wasm_path(Path::new(path))?;
    let wasm = fs::read(&wasm_path)
        .with_context(|| format!("failed to read built wasm at {}", wasm_path.display()))?;
    let metadata = load_metadata(&wasm)?;

    match select_test_target(&metadata.hooks, hook, route)? {
        TestTarget::Http { method, path } => run_http(&wasm, method, path)?,
        TestTarget::Cron {
            id,
            entry,
            schedule,
        } => run_cron(&wasm, id, entry, schedule)?,
    }

    Ok(())
}

fn run_http(wasm: &[u8], method: String, path: String) -> Result<()> {
    let registry = HookRegistry::default();
    registry.register_wasm(wasm)?;
    let response = dispatch(&registry, &HookRequest::new(method, path, "rack.local"));

    println!("HTTP {}", response.status);
    for (name, value) in response.headers {
        println!("{name}: {value}");
    }
    if !response.body.is_empty() {
        println!();
        print!("{}", String::from_utf8_lossy(&response.body));
    }
    Ok(())
}

fn run_cron(wasm: &[u8], id: String, entry: String, schedule: String) -> Result<()> {
    let event = CronEvent::new("local", id.clone(), schedule, unix_timestamp());
    run_cron_wasm_with_event(wasm, &entry, &event)?;
    println!("Cron {id} completed");
    Ok(())
}

fn unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}
