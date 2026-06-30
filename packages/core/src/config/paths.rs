use std::{env, path::PathBuf};

use super::backfill::BackfillError;

const CONFIG_FILE: &str = "config.toml";
const CONFIG_DIR: &str = "rack";
const CACHE_FILE: &str = "config.full.toml";
const CACHE_DIR: &str = "Rack";

pub fn config_path() -> Result<Option<PathBuf>, BackfillError> {
    let mut fallback = None;

    if let Some(path) = xdg_config_path() {
        if path.exists() {
            return Ok(Some(path));
        }
        fallback = Some(path);
    }

    if let Some(path) = home_config_path() {
        if path.exists() {
            return Ok(Some(path));
        }
        fallback.get_or_insert(path);
    }

    Ok(fallback)
}

pub fn cache_path() -> Result<PathBuf, BackfillError> {
    home()
        .map(|home| {
            home.join("Library")
                .join("Caches")
                .join(CACHE_DIR)
                .join(CACHE_FILE)
        })
        .ok_or(BackfillError::CacheDirectoryNotFound)
}

fn xdg_config_path() -> Option<PathBuf> {
    non_empty_var("XDG_CONFIG_HOME").map(|path| path.join(CONFIG_DIR).join(CONFIG_FILE))
}

fn home_config_path() -> Option<PathBuf> {
    home().map(|home| home.join(".config").join(CONFIG_DIR).join(CONFIG_FILE))
}

fn home() -> Option<PathBuf> {
    non_empty_var("HOME")
}

fn non_empty_var(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}
