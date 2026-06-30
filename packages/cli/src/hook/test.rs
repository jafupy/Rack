use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use rack_hooks::{
    dispatch, load_metadata, run_cron_wasm_with_event, CronEvent, HookRegistry, HookRequest,
    WasmHookEndpoint,
};

use super::build;

pub fn run(path: &str, hook: Option<&str>, route: Option<&str>) -> Result<()> {
    build::run(path)?;
    let wasm_path = built_wasm_path(Path::new(path))?;
    let wasm = fs::read(&wasm_path)
        .with_context(|| format!("failed to read built wasm at {}", wasm_path.display()))?;
    let metadata = load_metadata(&wasm)?;
    let target = select_test_target(&metadata.hooks, hook, route)?;

    match target {
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

enum TestTarget {
    Http {
        method: String,
        path: String,
    },
    Cron {
        id: String,
        entry: String,
        schedule: String,
    },
}

fn select_test_target(
    hooks: &[WasmHookEndpoint],
    hook: Option<&str>,
    route: Option<&str>,
) -> Result<TestTarget> {
    if hook.is_some() && route.is_some() {
        bail!("use either --hook or --route, not both");
    }

    if let Some(route) = route {
        return select_route(hooks, &normalize_route(route));
    }

    if let Some(id) = hook {
        return select_hook(hooks, id);
    }

    select_first(hooks)
}

fn select_route(hooks: &[WasmHookEndpoint], route: &str) -> Result<TestTarget> {
    hooks
        .iter()
        .find_map(|endpoint| match endpoint {
            WasmHookEndpoint::Http { method, path, .. } if path == route => {
                Some(TestTarget::Http {
                    method: method.clone(),
                    path: path.clone(),
                })
            }
            _ => None,
        })
        .ok_or_else(|| anyhow::anyhow!("unknown route `{route}`"))
}

fn select_hook(hooks: &[WasmHookEndpoint], id: &str) -> Result<TestTarget> {
    hooks
        .iter()
        .find_map(|endpoint| match endpoint {
            WasmHookEndpoint::Http {
                id: endpoint_id,
                method,
                path,
                ..
            } if endpoint_id == id => Some(TestTarget::Http {
                method: method.clone(),
                path: path.clone(),
            }),
            WasmHookEndpoint::Cron {
                id: endpoint_id,
                entry,
                schedule,
            } if endpoint_id == id => Some(TestTarget::Cron {
                id: endpoint_id.clone(),
                entry: entry.clone(),
                schedule: schedule.clone(),
            }),
            _ => None,
        })
        .ok_or_else(|| anyhow::anyhow!("unknown hook `{id}`"))
}

fn select_first(hooks: &[WasmHookEndpoint]) -> Result<TestTarget> {
    hooks
        .iter()
        .find_map(|endpoint| match endpoint {
            WasmHookEndpoint::Http { method, path, .. } => Some(TestTarget::Http {
                method: method.clone(),
                path: path.clone(),
            }),
            _ => None,
        })
        .or_else(|| {
            hooks.iter().find_map(|endpoint| match endpoint {
                WasmHookEndpoint::Cron {
                    id,
                    entry,
                    schedule,
                } => Some(TestTarget::Cron {
                    id: id.clone(),
                    entry: entry.clone(),
                    schedule: schedule.clone(),
                }),
                _ => None,
            })
        })
        .ok_or_else(|| anyhow::anyhow!("hook metadata contains no routes or crons"))
}

fn built_wasm_path(path: &Path) -> Result<PathBuf> {
    let release_dir = path.join("target/wasm32-unknown-unknown/release");
    let mut matches = fs::read_dir(&release_dir)
        .with_context(|| format!("missing build output directory {}", release_dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "wasm")
        })
        .collect::<Vec<_>>();
    matches.sort();

    match matches.as_slice() {
        [path] => Ok(path.clone()),
        [] => bail!("no wasm artifact found in {}", release_dir.display()),
        _ => bail!(
            "multiple wasm artifacts found in {}; cannot choose",
            release_dir.display()
        ),
    }
}

fn normalize_route(route: &str) -> String {
    if route.starts_with('/') {
        route.to_string()
    } else {
        format!("/{route}")
    }
}

fn unix_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}
