use anyhow::{bail, Result};
use rack_hooks::WasmHookEndpoint;

pub enum TestTarget {
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

pub fn select_test_target(
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

fn normalize_route(route: &str) -> String {
    if route.starts_with('/') {
        route.to_string()
    } else {
        format!("/{route}")
    }
}
