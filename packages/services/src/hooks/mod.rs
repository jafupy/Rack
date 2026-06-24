use std::{fs, path::PathBuf};

use rack_hooks::{load_metadata, HookRegistry, WasmHookEndpoint};
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

pub fn load_deployed(registry: &HookRegistry) -> Vec<HookSummary> {
    deployed_hook_dirs()
        .into_iter()
        .map(|dir| load_hook_dir(registry, dir))
        .collect()
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

fn load_hook_dir(registry: &HookRegistry, dir: PathBuf) -> HookSummary {
    let name = dir_name(&dir);
    let Some(wasm) = built_wasm(&dir) else {
        return HookSummary {
            name,
            routes: Vec::new(),
            crons: Vec::new(),
            errors: vec!["built wasm not found".to_string()],
        };
    };

    match fs::read(&wasm) {
        Ok(bytes) => summarize_wasm(registry, name, &bytes),
        Err(error) => HookSummary {
            name,
            routes: Vec::new(),
            crons: Vec::new(),
            errors: vec![error.to_string()],
        },
    }
}

fn summarize_wasm(registry: &HookRegistry, name: String, bytes: &[u8]) -> HookSummary {
    let metadata = load_metadata(bytes);
    let mut summary = match metadata {
        Ok(metadata) => summary_from_metadata(name, metadata.hooks),
        Err(error) => HookSummary {
            name,
            routes: Vec::new(),
            crons: Vec::new(),
            errors: vec![error.to_string()],
        },
    };

    if let Err(error) = registry.register_wasm(bytes) {
        summary.errors.push(error.to_string());
    }
    summary
}

fn summary_from_metadata(name: String, hooks: Vec<WasmHookEndpoint>) -> HookSummary {
    let mut routes = Vec::new();
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
            } => crons.push(HookCronSummary {
                schedule,
                hook: if id.is_empty() { entry } else { id },
            }),
        }
    }

    HookSummary {
        name,
        routes,
        crons,
        errors: Vec::new(),
    }
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
