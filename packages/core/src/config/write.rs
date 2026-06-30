use std::{fs, io, path::PathBuf};

use thiserror::Error;

use super::{
    backfill::BackfillError, cache::cache_config, format::format_config, paths::config_path,
    validate_config, Config, Service, ValidationErrors,
};

#[derive(Debug, Error)]
pub enum WriteError {
    #[error("terminal cannot be blank")]
    BlankTerminal,

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

    #[error("service `{0}` already exists")]
    ServiceAlreadyExists(String),

    #[error("unknown service `{0}`")]
    ServiceNotFound(String),

    #[error("edited service id `{service_id}` does not match target id `{target_id}`")]
    ServiceIdMismatch {
        target_id: String,
        service_id: String,
    },

    #[error("failed to cache config after write: {0}")]
    Cache(#[source] BackfillError),
}

pub fn set_terminal(config: &mut Config, terminal: impl Into<String>) -> Result<(), WriteError> {
    let terminal = terminal.into();
    if terminal.trim().is_empty() {
        return Err(WriteError::BlankTerminal);
    }

    config.terminal = terminal;
    validate_config(config)?;
    Ok(())
}

pub fn add_service(config: &mut Config, service: Service) -> Result<(), WriteError> {
    if config
        .services
        .iter()
        .any(|current| current.id == service.id)
    {
        return Err(WriteError::ServiceAlreadyExists(service.id));
    }

    config.services.push(service);
    if let Err(error) = validate_config(config) {
        config.services.pop();
        return Err(error.into());
    }

    Ok(())
}

pub fn replace_service(
    config: &mut Config,
    target_id: &str,
    service: Service,
) -> Result<(), WriteError> {
    if service.id != target_id {
        return Err(WriteError::ServiceIdMismatch {
            target_id: target_id.to_string(),
            service_id: service.id,
        });
    }

    let Some(index) = config
        .services
        .iter()
        .position(|current| current.id == target_id)
    else {
        return Err(WriteError::ServiceNotFound(target_id.to_string()));
    };

    let previous = std::mem::replace(&mut config.services[index], service);
    if let Err(error) = validate_config(config) {
        config.services[index] = previous;
        return Err(error.into());
    }

    Ok(())
}

pub fn remove_service(config: &mut Config, id: &str) -> Result<Service, WriteError> {
    let Some(index) = config.services.iter().position(|service| service.id == id) else {
        return Err(WriteError::ServiceNotFound(id.to_string()));
    };

    let removed = config.services.remove(index);
    if let Err(error) = validate_config(config) {
        config.services.insert(index, removed.clone());
        return Err(error.into());
    }

    Ok(removed)
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
