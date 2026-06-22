use std::{fs, io, path::PathBuf};

use thiserror::Error;

use super::{
    backfill::{cache_config, config_path, format_config, BackfillError},
    validate_config, Config, ValidationErrors,
};

#[derive(Debug, Error)]
pub enum WriteError {
    #[error("could not determine config path: {0}")]
    ConfigPath(#[source] BackfillError),

    #[error("could not determine config path")]
    ConfigPathNotFound,

    #[error("config is invalid: {0}")]
    Validate(#[from] ValidationErrors),

    #[error("failed to serialize config: {0}")]
    Format(#[source] BackfillError),

    #[error("failed to create config directory `{path}`: {source}")]
    CreateConfigDirectory { path: PathBuf, source: io::Error },

    #[error("failed to write config at `{path}`: {source}")]
    WriteConfig { path: PathBuf, source: io::Error },

    #[error("failed to cache config after write: {0}")]
    Cache(#[source] BackfillError),
}

pub fn save(config: &Config) -> Result<PathBuf, WriteError> {
    let path = config_path()
        .map_err(WriteError::ConfigPath)?
        .ok_or(WriteError::ConfigPathNotFound)?;
    save_at(path.clone(), config)?;
    cache_config(config).map_err(WriteError::Cache)?;
    Ok(path)
}

pub fn save_at(path: PathBuf, config: &Config) -> Result<(), WriteError> {
    validate_config(config)?;
    let output = format_config(config).map_err(WriteError::Format)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| WriteError::CreateConfigDirectory {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    fs::write(&path, output).map_err(|source| WriteError::WriteConfig { path, source })
}
