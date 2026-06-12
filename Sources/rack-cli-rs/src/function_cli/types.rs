use crate::Result;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(serde::Deserialize)]
pub(crate) struct FunctionManifest {
    pub(crate) name: String,
    pub(crate) version: String,
    #[serde(default)]
    pub(crate) route: BTreeMap<String, ManifestRoute>,
    #[serde(default)]
    pub(crate) cron: BTreeMap<String, ManifestCron>,
}

#[derive(serde::Deserialize)]
pub(crate) struct ManifestRoute {
    pub(crate) path: String,
    pub(crate) method: String,
    pub(crate) function: String,
}

#[derive(serde::Deserialize)]
pub(crate) struct ManifestCron {
    pub(crate) schedule: String,
    pub(crate) function: String,
}

pub(crate) fn read_function_manifest(path: &Path) -> Result<FunctionManifest> {
    let source = fs::read_to_string(path).map_err(|error| error.to_string())?;
    toml::from_str(&source).map_err(|error| format!("invalid manifest.toml: {error}"))
}

pub(crate) fn normalize_route_path(path: &str) -> String {
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
