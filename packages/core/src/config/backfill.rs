use std::{env, fs, io, path::PathBuf};
use thiserror::Error;

use super::{
    parse::{parse_full_config, parse_partial_config, parse_schema_version, ParseError},
    preamble, Config, Service,
};

const DEFAULT_CONFIG: &str = include_str!("../../public/default-config.toml");
const CONFIG_FILE_NAME: &str = "config.toml";
const CONFIG_DIR_NAME: &str = "rack";
const CACHE_CONFIG_FILE_NAME: &str = "config.full.toml";
const CACHE_DIR_NAME: &str = "Rack";

#[derive(Debug, Error)]
pub enum BackfillError {
    #[error("could not determine config directory: neither XDG_CONFIG_HOME nor HOME is set")]
    ConfigDirectoryNotFound,

    #[error("could not determine cache directory: HOME is not set")]
    CacheDirectoryNotFound,

    #[error("failed to read config at `{path}`: {source}")]
    ReadConfig { path: PathBuf, source: io::Error },

    #[error("failed to read legacy JSON config at `{path}`: {source}")]
    ReadLegacyConfig { path: PathBuf, source: io::Error },

    #[error("failed to parse legacy JSON config at `{path}`: {source}")]
    ParseLegacyJson {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("failed to create config directory `{path}` for migrated TOML config: {source}")]
    CreateMigratedConfigDirectory { path: PathBuf, source: io::Error },

    #[error("failed to write migrated TOML config at `{path}`: {source}")]
    WriteMigratedConfig { path: PathBuf, source: io::Error },

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

/// Loads the user config, backfills missing top-level fields from
/// `public/default-config.toml`, caches the effective config at
/// `~/Library/Caches/Rack/config.full.toml`, and returns the in-memory
/// backfilled config.
///
/// Existing TOML source config files are not rewritten. If the TOML source
/// config is missing and a legacy JSON config exists, it is migrated once to
/// TOML. If no source config exists, this uses the bundled default config and
/// caches the effective config without creating the source config file.
///
/// Config lookup order is:
/// 1. `$XDG_CONFIG_HOME/rack/config.toml`
/// 2. `$HOME/.config/rack/config.toml`
pub fn load() -> Result<Config, BackfillError> {
    let path = config_path()?.ok_or(BackfillError::ConfigDirectoryNotFound)?;
    backfill_at(path)
}

/// Returns the config file path that `load` will use.
///
/// If neither candidate exists, this returns the first candidate that can be
/// constructed. `load` will then use the default config without creating or
/// rewriting the source config file.
pub fn config_path() -> Result<Option<PathBuf>, BackfillError> {
    let candidates = config_path_candidates();

    Ok(candidates
        .iter()
        .find(|path| path.exists())
        .cloned()
        .or_else(|| candidates.into_iter().next()))
}

pub fn config_path_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(xdg_config_home) = env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty())
    {
        paths.push(
            PathBuf::from(xdg_config_home)
                .join(CONFIG_DIR_NAME)
                .join(CONFIG_FILE_NAME),
        );
    }

    if let Some(home) = env::var_os("HOME").filter(|value| !value.is_empty()) {
        paths.push(
            PathBuf::from(home)
                .join(".config")
                .join(CONFIG_DIR_NAME)
                .join(CONFIG_FILE_NAME),
        );
    }

    paths
}

pub fn backfill_at(path: PathBuf) -> Result<Config, BackfillError> {
    let cache_path = cache_path()?;
    backfill_at_with_cache_path(path, cache_path)
}

fn backfill_at_with_cache_path(
    config_path: PathBuf,
    cache_path: PathBuf,
) -> Result<Config, BackfillError> {
    let input = match fs::read_to_string(&config_path) {
        Ok(input) => input,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            super::legacy::migrate_if_needed(&config_path)?.unwrap_or_default()
        }
        Err(source) => {
            return Err(BackfillError::ReadConfig {
                path: config_path,
                source,
            })
        }
    };

    let config = backfill_str(&input)?;
    cache_config_at_with_source(cache_path, &config, &config_path)?;

    Ok(config)
}

pub fn cache_path() -> Result<PathBuf, BackfillError> {
    let Some(home) = env::var_os("HOME").filter(|value| !value.is_empty()) else {
        return Err(BackfillError::CacheDirectoryNotFound);
    };

    Ok(PathBuf::from(home)
        .join("Library")
        .join("Caches")
        .join(CACHE_DIR_NAME)
        .join(CACHE_CONFIG_FILE_NAME))
}

pub fn cache_config(config: &Config) -> Result<(), BackfillError> {
    cache_config_at(cache_path()?, config)
}

pub fn cache_config_at(path: PathBuf, config: &Config) -> Result<(), BackfillError> {
    cache_config_at_with_source(path, config, "unknown")
}

fn cache_config_at_with_source(
    path: PathBuf,
    config: &Config,
    source_path: impl AsRef<std::path::Path>,
) -> Result<(), BackfillError> {
    let output = format_cached_config(config, source_path.as_ref())?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| BackfillError::CreateCacheDirectory {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    fs::write(&path, output).map_err(|source| BackfillError::WriteCacheConfig { path, source })
}

pub fn backfill_str(input: &str) -> Result<Config, BackfillError> {
    let default = parse_full_config(DEFAULT_CONFIG)?;

    if input.trim().is_empty() {
        return Ok(default);
    }

    let mut config = parse_partial_config(input)?;
    config.schema_version = schema_version_or_default(input, default.schema_version)?;
    config.services = merge_services(config.services)?;

    Ok(config)
}

pub fn format_config(config: &Config) -> Result<String, BackfillError> {
    let toml = config_toml(config)?;

    Ok(format!(
        "{}\n\n{}",
        preamble::format_schema_header(config.schema_version),
        toml
    ))
}

pub fn format_cached_config(
    config: &Config,
    source_path: &std::path::Path,
) -> Result<String, BackfillError> {
    let toml = config_toml(config)?;

    Ok(format!(
        "{}\n\n{}",
        preamble::format_generated_preamble(source_path, config.schema_version),
        toml
    ))
}

fn config_toml(config: &Config) -> Result<String, BackfillError> {
    Ok(toml::to_string_pretty(config)?)
}

fn schema_version_or_default(input: &str, default: u8) -> Result<u8, ParseError> {
    match parse_schema_version(input) {
        Ok(version) => Ok(version),
        Err(ParseError::MissingSchemaHeader) => Ok(default),
        Err(error) => Err(error),
    }
}

fn merge_services(services: Vec<Service>) -> Result<Vec<Service>, BackfillError> {
    services
        .into_iter()
        .enumerate()
        .map(|(index, service)| merge_service(index, service))
        .collect()
}

fn merge_service(index: usize, service: Service) -> Result<Service, BackfillError> {
    let name = required_service_field(index, "name", service.name)?;

    Ok(Service {
        id: required_service_field(index, "id", service.id)?,
        host: if service.host.is_empty() {
            name.to_lowercase()
        } else {
            service.host
        },
        name,
        run: required_service_field(index, "run", service.run)?,
        working_dir: service.working_dir,
        auto_start: service.auto_start,
    })
}

fn required_service_field<T>(
    index: usize,
    field: &'static str,
    value: impl IntoRequired<T>,
) -> Result<T, BackfillError> {
    value
        .into_required()
        .ok_or(BackfillError::MissingServiceField { index, field })
}

trait IntoRequired<T> {
    fn into_required(self) -> Option<T>;
}

impl<T> IntoRequired<T> for Option<T> {
    fn into_required(self) -> Option<T> {
        self
    }
}

impl IntoRequired<String> for String {
    fn into_required(self) -> Option<String> {
        (!self.is_empty()).then_some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backfills_missing_top_level_fields() {
        let config = backfill_str(
            r#"# RACK:V1

terminal = "Alacritty"
"#,
        )
        .unwrap();

        assert_eq!(config.schema_version, 1);
        assert!(!config.use_standard_ports);
        assert_eq!(config.terminal, "Alacritty");
        assert_eq!(config.services.len(), 1);
    }

    #[test]
    fn backfills_only_safe_service_fields() {
        let config = backfill_str(
            r#"# RACK:V1

use_standard_ports = true
terminal = "Ghostty"

[[services]]
id = "service-1"
name = "API"
run = "cargo run"
"#,
        )
        .unwrap();

        let service = &config.services[0];
        assert_eq!(service.id, "service-1");
        assert_eq!(service.name, "API");
        assert_eq!(service.host, "api");
        assert_eq!(service.run, "cargo run");
        assert_eq!(service.working_dir, "~");
        assert!(!service.auto_start);
    }

    #[test]
    fn rejects_services_missing_required_fields() {
        let error = backfill_str(
            r#"# RACK:V1

use_standard_ports = true
terminal = "Ghostty"

[[services]]
name = "API"
run = "cargo run"
"#,
        )
        .unwrap_err();

        assert!(matches!(
            error,
            BackfillError::MissingServiceField {
                index: 0,
                field: "id"
            }
        ));
    }

    #[test]
    fn caches_backfilled_config_without_rewriting_source_config() {
        let config_path = unique_temp_path("config");
        let cache_path = unique_temp_path("cache");
        let source = "terminal = \"Terminal.app\"\n";
        fs::write(&config_path, source).unwrap();

        let config = backfill_at_with_cache_path(config_path.clone(), cache_path.clone()).unwrap();
        let source_after_backfill = fs::read_to_string(&config_path).unwrap();
        let cached = fs::read_to_string(&cache_path).unwrap();

        assert_eq!(config.terminal, "Terminal.app");
        assert_eq!(source_after_backfill, source);
        assert!(cached.starts_with("# RACK:GENERATED"));
        assert!(cached.contains(&format!("# Source config is {}", config_path.display())));
        assert!(cached.contains("# Do not edit this file directly."));
        assert!(cached.contains("# RACK:V1"));
        assert!(cached.contains("use_standard_ports = false"));
        assert!(cached.contains("terminal = \"Terminal.app\""));
        assert!(cached.contains("[[services]]"));

        let _ = fs::remove_file(config_path);
        let _ = fs::remove_file(cache_path);
    }

    #[test]
    fn existing_source_toml_wins_over_legacy_json() {
        let config_path = unique_temp_path("existing-config");
        let legacy_path = unique_temp_path("ignored-legacy-config-json");
        let cache_path = unique_temp_path("existing-cache");
        let source = r#"# RACK:V1

use_standard_ports = true
terminal = "Terminal.app"

[[services]]
id = "toml"
name = "TOML"
host = "toml"
run = "cargo run"
working_dir = "~"
auto_start = false
"#;
        fs::write(&config_path, source).unwrap();
        fs::write(
            &legacy_path,
            r#"{"servers":[{"id":"json","name":"JSON","command":"npm","arguments":"run dev"}]}"#,
        )
        .unwrap();

        let config = backfill_at_with_cache_path(config_path.clone(), cache_path.clone()).unwrap();
        let source_after_backfill = fs::read_to_string(&config_path).unwrap();

        assert_eq!(source_after_backfill, source);
        assert_eq!(config.terminal, "Terminal.app");
        assert_eq!(config.services.len(), 1);
        assert_eq!(config.services[0].id, "toml");

        let _ = fs::remove_file(config_path);
        let _ = fs::remove_file(legacy_path);
        let _ = fs::remove_file(cache_path);
    }

    fn unique_temp_path(label: &str) -> PathBuf {
        let mut path = env::temp_dir();
        path.push(format!(
            "rack-config-backfill-test-{}-{}-{label}.toml",
            std::process::id(),
            std::thread::current().name().unwrap_or("unnamed")
        ));
        path
    }
}
