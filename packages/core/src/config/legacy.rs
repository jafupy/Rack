use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use super::{backfill::format_config, backfill::BackfillError, Config, Service};

const CONFIG_DIR_NAME: &str = "rack";

pub(crate) fn migrate_if_needed(destination: &Path) -> Result<Option<String>, BackfillError> {
    migrate_from_candidates(destination, legacy_json_config_candidates())
}

pub(crate) fn migrate_from_candidates(
    destination: &Path,
    candidates: Vec<PathBuf>,
) -> Result<Option<String>, BackfillError> {
    let Some(source) = candidates.into_iter().find(|candidate| candidate.is_file()) else {
        return Ok(None);
    };

    let input =
        fs::read_to_string(&source).map_err(|source_error| BackfillError::ReadLegacyConfig {
            path: source.clone(),
            source: source_error,
        })?;
    let legacy: LegacyPersistedConfiguration =
        serde_json::from_str(&input).map_err(|source_error| BackfillError::ParseLegacyJson {
            path: source.clone(),
            source: source_error,
        })?;
    let output = format_config(&legacy.into_config())?;

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|source| {
            BackfillError::CreateMigratedConfigDirectory {
                path: parent.to_path_buf(),
                source,
            }
        })?;
    }

    fs::write(destination, &output).map_err(|source| BackfillError::WriteMigratedConfig {
        path: destination.to_path_buf(),
        source,
    })?;

    Ok(Some(output))
}

fn legacy_json_config_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(xdg_config_home) = env::var_os("XDG_CONFIG_HOME").filter(|value| !value.is_empty())
    {
        paths.push(
            PathBuf::from(xdg_config_home)
                .join(CONFIG_DIR_NAME)
                .join("config.json"),
        );
    }

    if let Some(home) = env::var_os("HOME").filter(|value| !value.is_empty()) {
        let home = PathBuf::from(home);
        paths.push(
            home.join(".config")
                .join(CONFIG_DIR_NAME)
                .join("config.json"),
        );
        paths.push(home.join(".config").join("server-bar").join("config.json"));
        paths.push(
            home.join("Library")
                .join("Application Support")
                .join("ServerBar")
                .join("servers.json"),
        );
    }

    paths
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct LegacyPersistedConfiguration {
    servers: Vec<LegacyServerConfiguration>,
}

impl LegacyPersistedConfiguration {
    fn into_config(self) -> Config {
        Config {
            services: self
                .servers
                .into_iter()
                .map(LegacyServerConfiguration::into_service)
                .collect(),
            ..Config::default()
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct LegacyServerConfiguration {
    id: String,
    name: String,
    command: String,
    arguments: String,
    working_directory: String,
    auto_start: bool,
    custom_domain: String,
}

impl Default for LegacyServerConfiguration {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: "New Server".to_string(),
            command: String::new(),
            arguments: String::new(),
            working_directory: String::new(),
            auto_start: false,
            custom_domain: String::new(),
        }
    }
}

impl LegacyServerConfiguration {
    fn into_service(self) -> Service {
        let host = legacy_route_subdomain(&self);
        let run = legacy_run_command(&self);

        Service {
            id: self.id,
            name: self.name,
            host,
            run,
            working_dir: if self.working_directory.trim().is_empty() {
                "~".to_string()
            } else {
                self.working_directory
            },
            auto_start: self.auto_start,
        }
    }
}

fn legacy_run_command(server: &LegacyServerConfiguration) -> String {
    match (server.command.trim(), server.arguments.trim()) {
        ("", arguments) => arguments.to_string(),
        (command, "") => command.to_string(),
        (command, arguments) => format!("{command} {arguments}"),
    }
}

fn legacy_route_subdomain(server: &LegacyServerConfiguration) -> String {
    let raw = if server.custom_domain.trim().is_empty() {
        &server.name
    } else {
        &server.custom_domain
    };
    let trimmed = raw.trim().to_lowercase().replace(' ', "-");
    trimmed
        .strip_suffix(".localhost")
        .unwrap_or(&trimmed)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_legacy_json_config_to_source_toml_when_source_toml_is_missing() {
        let config_path = unique_temp_path("migrated-config");
        let legacy_path = unique_temp_path("legacy-config-json");
        fs::write(
            &legacy_path,
            r#"{
  "servers": [
    {
      "id": "api",
      "name": "API Server",
      "command": "bun",
      "arguments": "dev --host 127.0.0.1",
      "workingDirectory": "/Users/jafu/Projects/api",
      "autoStart": true,
      "customDomain": "api.localhost"
    }
  ]
}"#,
        )
        .unwrap();

        let migrated = migrate_from_candidates(
            &config_path,
            vec![unique_temp_path("missing-legacy"), legacy_path.clone()],
        )
        .unwrap()
        .unwrap();
        let source = fs::read_to_string(&config_path).unwrap();

        assert_eq!(source, migrated);
        assert!(source.starts_with("# RACK:V1\n\n"));
        assert!(source.contains("terminal = \"Ghostty\""));
        assert!(source.contains("[[services]]"));
        assert!(source.contains("id = \"api\""));
        assert!(source.contains("name = \"API Server\""));
        assert!(source.contains("host = \"api\""));
        assert!(source.contains("run = \"bun dev --host 127.0.0.1\""));
        assert!(source.contains("working_dir = \"/Users/jafu/Projects/api\""));
        assert!(source.contains("auto_start = true"));

        let _ = fs::remove_file(config_path);
        let _ = fs::remove_file(legacy_path);
    }

    fn unique_temp_path(label: &str) -> PathBuf {
        let mut path = env::temp_dir();
        path.push(format!(
            "rack-config-legacy-test-{}-{}-{label}.toml",
            std::process::id(),
            std::thread::current().name().unwrap_or("unnamed")
        ));
        path
    }
}
