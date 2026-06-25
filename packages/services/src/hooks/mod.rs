mod scheduler;

use std::{fs, path::PathBuf, sync::Arc};

use rack_hooks::{load_metadata, HookRegistry, WasmHookEndpoint};

pub use scheduler::{CronHook, HookScheduler};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookSummary {
    pub name: String,
    pub routes: Vec<HookRouteSummary>,
    pub crons: Vec<HookCronSummary>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookRouteSummary {
    pub method: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookCronSummary {
    pub schedule: String,
    pub hook: String,
}

pub struct DeployedHooks {
    pub summaries: Vec<HookSummary>,
    pub crons: Vec<CronHook>,
}

pub fn load_deployed(registry: &HookRegistry) -> DeployedHooks {
    let mut summaries = Vec::new();
    let mut crons = Vec::new();

    for dir in deployed_hook_dirs() {
        let loaded = load_hook_dir(registry, dir);
        summaries.push(loaded.summary);
        crons.extend(loaded.crons);
    }

    DeployedHooks { summaries, crons }
}

fn deployed_hook_dirs() -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("HOME") else {
        return Vec::new();
    };
    let root = PathBuf::from(home).join(".rack/hooks");
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };

    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect()
}

struct LoadedHook {
    summary: HookSummary,
    crons: Vec<CronHook>,
}

fn load_hook_dir(registry: &HookRegistry, dir: PathBuf) -> LoadedHook {
    let name = dir_name(&dir);
    let Some(wasm) = built_wasm(&dir) else {
        return LoadedHook {
            summary: HookSummary {
                name,
                routes: Vec::new(),
                crons: Vec::new(),
                errors: vec!["built wasm not found".to_string()],
            },
            crons: Vec::new(),
        };
    };

    match fs::read(&wasm) {
        Ok(bytes) => summarize_wasm(registry, name, bytes),
        Err(error) => LoadedHook {
            summary: HookSummary {
                name,
                routes: Vec::new(),
                crons: Vec::new(),
                errors: vec![error.to_string()],
            },
            crons: Vec::new(),
        },
    }
}

fn summarize_wasm(registry: &HookRegistry, name: String, bytes: Vec<u8>) -> LoadedHook {
    let metadata = load_metadata(&bytes);
    let wasm = Arc::new(bytes);
    let (mut summary, crons) = match metadata {
        Ok(metadata) => summary_from_metadata(name, metadata.hooks, wasm.clone()),
        Err(error) => (
            HookSummary {
                name,
                routes: Vec::new(),
                crons: Vec::new(),
                errors: vec![error.to_string()],
            },
            Vec::new(),
        ),
    };

    if let Err(error) = registry.register_wasm(wasm.as_slice()) {
        summary.errors.push(error.to_string());
    }
    LoadedHook { summary, crons }
}

fn summary_from_metadata(
    name: String,
    hooks: Vec<WasmHookEndpoint>,
    wasm: Arc<Vec<u8>>,
) -> (HookSummary, Vec<CronHook>) {
    let mut routes = Vec::new();
    let mut cron_summaries = Vec::new();
    let mut crons = Vec::new();

    for hook in hooks {
        match hook {
            WasmHookEndpoint::Http { method, path, .. } => {
                routes.push(HookRouteSummary { method, path });
            }
            WasmHookEndpoint::Cron {
                id,
                schedule,
                entry,
            } => {
                let hook = if id.is_empty() {
                    entry.clone()
                } else {
                    id.clone()
                };
                cron_summaries.push(HookCronSummary {
                    schedule: schedule.clone(),
                    hook: hook.clone(),
                });
                crons.push(CronHook {
                    package: name.clone(),
                    id: hook,
                    schedule,
                    entry,
                    wasm: wasm.clone(),
                });
            }
        }
    }

    (
        HookSummary {
            name,
            routes,
            crons: cron_summaries,
            errors: Vec::new(),
        },
        crons,
    )
}

fn built_wasm(dir: &PathBuf) -> Option<PathBuf> {
    let deps = dir.join("target/wasm32-unknown-unknown/release");
    fs::read_dir(deps)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|ext| ext == "wasm"))
}

fn dir_name(dir: &PathBuf) -> String {
    dir.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string()
}
