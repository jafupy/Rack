use std::{fs, io, path::PathBuf};

use thiserror::Error;

use super::{
    cache::write_cache,
    parse::{parse_full_config, parse_partial_config, parse_schema_version, ParseError},
    paths::{cache_path, config_path},
    Config, Service,
};

const DEFAULT_CONFIG: &str = include_str!("../../public/default-config.toml");

#[derive(Debug, Error)]
pub enum BackfillError {
    #[error("could not determine config directory: neither XDG_CONFIG_HOME nor HOME is set")]
    ConfigDirectoryNotFound,

    #[error("could not determine cache directory: HOME is not set")]
    CacheDirectoryNotFound,

    #[error("failed to read config at `{path}`: {source}")]
    ReadConfig { path: PathBuf, source: io::Error },

    #[error("failed to create cache directory `{path}`: {source}")]
    CreateCacheDirectory { path: PathBuf, source: io::Error },

    #[error("failed to write cached config at `{path}`: {source}")]
    WriteCacheConfig { path: PathBuf, source: io::Error },

    #[error("service at index {index} is missing required field `{field}`")]
    MissingServiceField { index: usize, field: &'static str },

    #[error(transparent)]
    Parse(#[from] ParseError),

    #[error("failed to serialize backfilled TOML config: {0}")]
    SerializeToml(#[from] toml::ser::Error),
}

pub fn load() -> Result<Config, BackfillError> {
    let config_path = config_path()?.ok_or(BackfillError::ConfigDirectoryNotFound)?;
    let cache_path = cache_path()?;
    let source = read_source(&config_path)?;
    let config = backfill_str(&source)?;
    write_cache(&cache_path, &config, &config_path)?;
    Ok(config)
}

pub fn backfill_str(input: &str) -> Result<Config, BackfillError> {
    let default = parse_full_config(DEFAULT_CONFIG)?;
    if input.trim().is_empty() {
        return Ok(default);
    }

    let mut config = parse_partial_config(input)?;
    config.schema_version = schema_version(input, default.schema_version)?;
    config.services = config
        .services
        .into_iter()
        .enumerate()
        .map(backfill_service)
        .collect::<Result<_, _>>()?;
    Ok(config)
}

fn read_source(path: &PathBuf) -> Result<String, BackfillError> {
    match fs::read_to_string(path) {
        Ok(input) => Ok(input),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(source) => Err(BackfillError::ReadConfig {
            path: path.clone(),
            source,
        }),
    }
}

fn schema_version(input: &str, default: u8) -> Result<u8, ParseError> {
    match parse_schema_version(input) {
        Ok(version) => Ok(version),
        Err(ParseError::MissingSchemaHeader) => Ok(default),
        Err(error) => Err(error),
    }
}

fn backfill_service((index, service): (usize, Service)) -> Result<Service, BackfillError> {
    let name = required(index, "name", service.name)?;
    let host = if service.host.is_empty() {
        name.to_lowercase()
    } else {
        service.host
    };

    Ok(Service {
        id: required(index, "id", service.id)?,
        name,
        host,
        run: required(index, "run", service.run)?,
        working_dir: service.working_dir,
        auto_start: service.auto_start,
    })
}

fn required(index: usize, field: &'static str, value: String) -> Result<String, BackfillError> {
    (!value.is_empty())
        .then_some(value)
        .ok_or(BackfillError::MissingServiceField { index, field })
}
