use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const KEY_FILES: &[&str] = &[
    "package.json",
    "Cargo.toml",
    "go.mod",
    "Package.swift",
    "Gemfile",
    "pyproject.toml",
    "requirements.txt",
    "pnpm-lock.yaml",
    "yarn.lock",
    "bun.lockb",
    "Makefile",
    "vite.config.ts",
    "vite.config.js",
    "vite.config.mts",
    "astro.config.ts",
    "astro.config.js",
    "astro.config.mjs",
    "manage.py",
    "artisan",
];

#[derive(serde::Deserialize)]
struct DetectPayload {
    directory: PathBuf,
}

#[derive(Clone)]
struct ProjectManifest {
    files: BTreeSet<String>,
    contents: BTreeMap<String, String>,
}

#[derive(Serialize)]
struct DevCommand {
    command: String,
    env: BTreeMap<String, String>,
    name: Option<String>,
    #[serde(rename = "portFlag")]
    port_flag: Option<String>,
}

pub(crate) fn dev_command(command_type: &str, payload: &Value) -> Option<Value> {
    if command_type != "dev.detect" {
        return None;
    }

    Some(match detect_command(payload) {
        Ok(command) => serde_json::json!({
            "type": "dev.detect",
            "payload": command,
        }),
        Err(message) => serde_json::json!({
            "type": "error",
            "message": message,
        }),
    })
}

fn detect_command(payload: &Value) -> Result<Option<DevCommand>, String> {
    let payload: DetectPayload =
        serde_json::from_value(payload.clone()).map_err(|error| error.to_string())?;
    Ok(detect(&ProjectManifest::load(&payload.directory)))
}

impl ProjectManifest {
    fn load(directory: &Path) -> Self {
        let files = std::fs::read_dir(directory)
            .ok()
            .into_iter()
            .flat_map(|entries| entries.flatten())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect();

        let mut contents = BTreeMap::new();
        for name in KEY_FILES {
            if let Ok(text) = std::fs::read_to_string(directory.join(name)) {
                contents.insert((*name).to_string(), text);
            }
        }

        Self { files, contents }
    }

    fn has(&self, file: &str) -> bool {
        self.files.contains(file)
    }

    fn has_prefix(&self, prefix: &str) -> bool {
        self.files.iter().any(|file| file.starts_with(prefix))
    }

    fn content(&self, file: &str) -> Option<&str> {
        self.contents.get(file).map(String::as_str)
    }
}

fn detect(manifest: &ProjectManifest) -> Option<DevCommand> {
    if manifest.has_prefix("vite.config.") {
        return Some(command(
            format!("{} exec vite", package_manager(manifest)),
            Some("--port"),
        ));
    }
    if manifest.has_prefix("astro.config.") {
        return Some(command(
            format!("{} run dev", package_manager(manifest)),
            Some("--port"),
        ));
    }
    if let Some(command) = node_command(manifest) {
        return Some(command);
    }
    if manifest.has("Package.swift") {
        return Some(command("swift run", None));
    }
    if manifest.has("Cargo.toml") {
        return Some(command("cargo run", None));
    }
    if manifest.has("go.mod") {
        return Some(command("go run .", None));
    }
    if manifest.has("manage.py") {
        return Some(command("python manage.py runserver", None));
    }
    if manifest
        .content("Gemfile")
        .is_some_and(|gemfile| gemfile.contains("rails"))
    {
        return Some(command("rails server", Some("-p")));
    }
    if manifest.has("artisan") {
        return Some(command("php artisan serve", Some("--port")));
    }
    if manifest
        .content("Makefile")
        .is_some_and(|makefile| makefile.contains("\ndev:") || makefile.starts_with("dev:"))
    {
        return Some(command("make dev", None));
    }
    None
}

fn node_command(manifest: &ProjectManifest) -> Option<DevCommand> {
    let package_json = manifest.content("package.json")?;
    let json: Value = serde_json::from_str(package_json).ok()?;
    let scripts = json.get("scripts")?.as_object()?;
    let package_manager = package_manager(manifest);
    ["dev", "start", "serve"]
        .iter()
        .find(|script| scripts.contains_key(**script))
        .map(|script| command(format!("{package_manager} run {script}"), None))
}

fn command(text: impl Into<String>, port_flag: Option<&str>) -> DevCommand {
    DevCommand {
        command: text.into(),
        env: BTreeMap::new(),
        name: None,
        port_flag: port_flag.map(str::to_string),
    }
}

fn package_manager(manifest: &ProjectManifest) -> &'static str {
    if manifest.has("bun.lockb") {
        "bun"
    } else if manifest.has("pnpm-lock.yaml") {
        "pnpm"
    } else if manifest.has("yarn.lock") {
        "yarn"
    } else {
        "npm"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(files: &[&str], contents: &[(&str, &str)]) -> ProjectManifest {
        ProjectManifest {
            files: files.iter().map(|file| (*file).to_string()).collect(),
            contents: contents
                .iter()
                .map(|(name, text)| ((*name).to_string(), (*text).to_string()))
                .collect(),
        }
    }

    #[test]
    fn vite_prefers_installed_package_manager() {
        let manifest = manifest(&["vite.config.ts", "pnpm-lock.yaml"], &[]);
        let command = detect(&manifest).unwrap();
        assert_eq!(command.command, "pnpm exec vite");
        assert_eq!(command.port_flag.as_deref(), Some("--port"));
    }

    #[test]
    fn node_scripts_are_detected_in_priority_order() {
        let manifest = manifest(
            &["package.json", "bun.lockb"],
            &[(
                "package.json",
                r#"{"scripts":{"start":"node server.js","dev":"vite"}}"#,
            )],
        );
        let command = detect(&manifest).unwrap();
        assert_eq!(command.command, "bun run dev");
        assert_eq!(command.port_flag, None);
    }

    #[test]
    fn rails_requires_a_rails_gemfile() {
        let manifest = manifest(&["Gemfile"], &[("Gemfile", "gem 'rails'\n")]);
        let command = detect(&manifest).unwrap();
        assert_eq!(command.command, "rails server");
        assert_eq!(command.port_flag.as_deref(), Some("-p"));
    }
}
