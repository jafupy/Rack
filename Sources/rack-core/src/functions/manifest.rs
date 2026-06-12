use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub(super) struct FunctionRoute {
    pub(super) package: String,
    pub(super) id: String,
    pub(super) path: String,
    pub(super) method: String,
    pub(super) function: String,
    pub(super) wasm_path: PathBuf,
}

#[derive(Clone, Debug)]
pub(super) struct FunctionRouteMatch {
    pub(super) route: FunctionRoute,
    pub(super) request_path: String,
}

#[derive(Clone, Debug)]
pub(super) struct FunctionCron {
    pub(super) package: String,
    pub(super) id: String,
    pub(super) schedule: String,
    pub(super) function: String,
    pub(super) wasm_path: PathBuf,
}

#[derive(Clone, Debug)]
pub(super) struct FunctionPackage {
    pub(super) name: String,
    pub(super) version: String,
    pub(super) root: PathBuf,
    pub(super) routes: Vec<FunctionRoute>,
    pub(super) crons: Vec<FunctionCron>,
    pub(super) errors: Vec<String>,
}

#[derive(serde::Deserialize)]
struct Manifest {
    name: String,
    version: String,
    #[serde(default)]
    route: BTreeMap<String, ManifestRoute>,
    #[serde(default)]
    cron: BTreeMap<String, ManifestCron>,
}

#[derive(serde::Deserialize)]
struct ManifestRoute {
    path: String,
    method: String,
    function: String,
}

#[derive(serde::Deserialize)]
struct ManifestCron {
    schedule: String,
    function: String,
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn functions_dir() -> PathBuf {
    home_dir().join(".rack").join("functions")
}

pub(super) fn normalize_route_path(path: &str) -> String {
    let trimmed = path.trim();
    let with_leading = if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    };
    if with_leading.len() > 1 {
        with_leading.trim_end_matches('/').to_string()
    } else {
        with_leading
    }
}

fn parse_manifest(root: &Path) -> FunctionPackage {
    let manifest_path = root.join("manifest.toml");
    let wasm_path = root.join("functions.wasm");
    let mut package = FunctionPackage {
        name: root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string(),
        version: "0.0.0".to_string(),
        root: root.to_path_buf(),
        routes: Vec::new(),
        crons: Vec::new(),
        errors: Vec::new(),
    };

    if !wasm_path.is_file() {
        package.errors.push("missing functions.wasm".to_string());
    }

    let source = match std::fs::read_to_string(&manifest_path) {
        Ok(source) => source,
        Err(_) => {
            package.errors.push("missing manifest.toml".to_string());
            return package;
        }
    };

    let manifest: Manifest = match toml::from_str(&source) {
        Ok(manifest) => manifest,
        Err(error) => {
            package
                .errors
                .push(format!("invalid manifest.toml: {error}"));
            return package;
        }
    };

    package.name = manifest.name;
    package.version = manifest.version;

    for (id, route) in manifest.route {
        let normalized = normalize_route_path(&route.path);
        if normalized == "/" || normalized.starts_with("/_") {
            package
                .errors
                .push(format!("route '{id}' uses reserved path '{normalized}'"));
            continue;
        }
        if let Err(message) = crate::functions::routing::validate_route_path(&normalized) {
            package.errors.push(format!("route '{id}' {message}"));
            continue;
        }

        package.routes.push(FunctionRoute {
            package: package.name.clone(),
            id,
            path: normalized,
            method: route.method.to_uppercase(),
            function: route.function,
            wasm_path: wasm_path.clone(),
        });
    }

    for (id, cron) in manifest.cron {
        package.crons.push(FunctionCron {
            package: package.name.clone(),
            id,
            schedule: cron.schedule,
            function: cron.function,
            wasm_path: wasm_path.clone(),
        });
    }

    if package.routes.is_empty() && package.crons.is_empty() {
        package
            .errors
            .push("manifest has no routes or crons".to_string());
    }

    package
}

pub(super) fn load_functions() -> Vec<FunctionPackage> {
    let dir = functions_dir();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut packages: Vec<_> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .map(|path| parse_manifest(&path))
        .collect();
    packages.sort_by(|left, right| left.name.cmp(&right.name));
    packages
}

pub(crate) fn function_snapshot_json() -> serde_json::Value {
    let packages = load_functions();
    let mut claimed_routes: Vec<(String, String, String)> = Vec::new();
    let functions: Vec<_> = packages
        .into_iter()
        .map(|mut package| {
            for route in &package.routes {
                let key = (
                    route.method.clone(),
                    route.path.clone(),
                    package.name.clone(),
                );
                if claimed_routes
                    .iter()
                    .any(|(method, path, _)| method == &route.method && path == &route.path)
                {
                    package.errors.push(format!(
                        "route conflict for {} {}",
                        route.method, route.path
                    ));
                } else {
                    claimed_routes.push(key);
                }
            }
            serde_json::json!({
                "name": package.name,
                "version": package.version,
                "root": package.root,
                "routes": package.routes.iter().map(|route| serde_json::json!({
                    "id": route.id,
                    "path": route.path,
                    "method": route.method,
                    "function": route.function,
                })).collect::<Vec<_>>(),
                "crons": package.crons.iter().map(|cron| serde_json::json!({
                    "id": cron.id,
                    "schedule": cron.schedule,
                    "function": cron.function,
                })).collect::<Vec<_>>(),
                "errors": package.errors,
            })
        })
        .collect();

    serde_json::json!(functions)
}
