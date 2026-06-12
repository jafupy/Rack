use crate::{send, Result};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

struct DetectedCommand {
    command: String,
    port_flag: Option<String>,
}

pub(crate) fn cmd_dev() -> Result<()> {
    let dir = env::current_dir().map_err(|error| error.to_string())?;
    let Some(detected) = detect_command(&dir) else {
        let name = dir
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("current directory");
        println!("rack: couldn't detect a dev command in {name}");
        println!("      supported: Node/Vite/Swift/Rust/Go/Django/Rails/Laravel/Make");
        std::process::exit(1);
    };

    let name = infer_name(&dir);
    println!("rack: detected  → {}", detected.command);
    println!("rack: name      → {name}");
    println!("rack: sending to Rack.app...");

    let mut payload = json!({
        "name": name,
        "command": detected.command,
        "workingDirectory": dir.to_string_lossy(),
        "environment": {},
    });
    if let Some(port_flag) = detected.port_flag {
        payload["portFlag"] = Value::String(port_flag);
    }

    let reply = send(&json!({ "type": "register", "payload": payload }))?;
    if let Some(url) = reply
        .get("payload")
        .and_then(|payload| payload.get("url"))
        .and_then(Value::as_str)
    {
        println!();
        println!("✓ {name}");
        println!("  {url}");
    } else if reply.get("type").and_then(Value::as_str) == Some("error") {
        let message = reply
            .get("payload")
            .and_then(Value::as_str)
            .unwrap_or("unknown error");
        return Err(format!("rack error: {message}"));
    }

    Ok(())
}

fn detect_command(directory: &Path) -> Option<DetectedCommand> {
    let files = fs::read_dir(directory)
        .ok()?
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect::<HashSet<_>>();

    let has = |name: &str| files.contains(name);
    let content = |name: &str| fs::read_to_string(directory.join(name)).ok();
    let package_manager = || {
        if has("bun.lockb") {
            "bun"
        } else if has("pnpm-lock.yaml") {
            "pnpm"
        } else if has("yarn.lock") {
            "yarn"
        } else {
            "npm"
        }
    };

    if files.iter().any(|file| file.starts_with("vite.config.")) {
        return Some(DetectedCommand {
            command: format!("{} exec vite", package_manager()),
            port_flag: Some("--port".to_string()),
        });
    }

    if files.iter().any(|file| file.starts_with("astro.config.")) {
        return Some(DetectedCommand {
            command: format!("{} run dev", package_manager()),
            port_flag: Some("--port".to_string()),
        });
    }

    if let Some(package_json) = content("package.json") {
        if let Ok(json) = serde_json::from_str::<Value>(&package_json) {
            if let Some(scripts) = json.get("scripts").and_then(Value::as_object) {
                let is_next = json
                    .get("dependencies")
                    .and_then(Value::as_object)
                    .is_some_and(|deps| deps.contains_key("next"))
                    || json
                        .get("devDependencies")
                        .and_then(Value::as_object)
                        .is_some_and(|deps| deps.contains_key("next"));
                for script in ["dev", "start", "serve"] {
                    if scripts.contains_key(script) {
                        return Some(DetectedCommand {
                            command: format!("{} run {script}", package_manager()),
                            port_flag: is_next.then(|| "-p".to_string()),
                        });
                    }
                }
            }
        }
    }

    if has("Package.swift") {
        return Some(DetectedCommand {
            command: "swift run".to_string(),
            port_flag: None,
        });
    }
    if has("Cargo.toml") {
        return Some(DetectedCommand {
            command: "cargo run".to_string(),
            port_flag: None,
        });
    }
    if has("go.mod") {
        return Some(DetectedCommand {
            command: "go run .".to_string(),
            port_flag: None,
        });
    }
    if has("manage.py") {
        return Some(DetectedCommand {
            command: "python manage.py runserver".to_string(),
            port_flag: None,
        });
    }
    if content("Gemfile").is_some_and(|gemfile| gemfile.contains("rails")) {
        return Some(DetectedCommand {
            command: "rails server".to_string(),
            port_flag: Some("-p".to_string()),
        });
    }
    if has("artisan") {
        return Some(DetectedCommand {
            command: "php artisan serve".to_string(),
            port_flag: Some("--port".to_string()),
        });
    }
    if content("Makefile")
        .is_some_and(|makefile| makefile.starts_with("dev:") || makefile.contains("\ndev:"))
    {
        return Some(DetectedCommand {
            command: "make dev".to_string(),
            port_flag: None,
        });
    }

    None
}

fn infer_name(directory: &Path) -> String {
    let mut base = directory
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("project")
        .to_string();

    if let Ok(remote) = capture(
        "git",
        &[
            "-C",
            path_str_lossy(directory).as_str(),
            "remote",
            "get-url",
            "origin",
        ],
        directory,
    ) {
        if let Some(last) = remote.trim().rsplit('/').next() {
            if !last.is_empty() {
                base = last.trim_end_matches(".git").to_string();
            }
        }
    } else if let Ok(package_json) = fs::read_to_string(directory.join("package.json")) {
        if let Ok(json) = serde_json::from_str::<Value>(&package_json) {
            if let Some(name) = json.get("name").and_then(Value::as_str) {
                base = name.rsplit('/').next().unwrap_or(name).to_string();
            }
        }
    }

    sanitize(&base)
}

fn capture(command: &str, args: &[&str], directory: &Path) -> Result<String> {
    let output = Command::new(command)
        .args(args)
        .current_dir(directory)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| error.to_string())?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("{command} {} failed", args.join(" "))
        } else {
            stderr
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn sanitize(value: &str) -> String {
    let mut result = String::new();
    let mut previous_dash = false;
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            result.push(character);
            previous_dash = false;
        } else if !previous_dash {
            result.push('-');
            previous_dash = true;
        }
    }
    let result = result.trim_matches('-').to_string();
    if result.is_empty() {
        "rack-function".to_string()
    } else {
        result
    }
}

fn path_str_lossy(path: &Path) -> String {
    path.to_string_lossy().to_string()
}
