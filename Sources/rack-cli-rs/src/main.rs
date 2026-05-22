use serde_json::{json, Value};
use std::collections::HashSet;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

type Result<T> = std::result::Result<T, String>;

struct DetectedCommand {
    command: String,
    port_flag: Option<String>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("rack: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.first().map(String::as_str) {
        Some("dev") => cmd_dev(),
        Some("ls" | "list") => cmd_ls(),
        Some("start") => {
            let name = args.get(1).ok_or("Usage: rack start <name>")?;
            send(&json!({ "type": "start", "payload": name }))?;
            println!("✓ started {name}");
            Ok(())
        }
        Some("stop") => {
            let name = args.get(1).ok_or("Usage: rack stop <name>")?;
            send(&json!({ "type": "stop", "payload": name }))?;
            println!("✓ stopped {name}");
            Ok(())
        }
        Some("rm" | "remove") => {
            let name = args.get(1).ok_or("Usage: rack rm <name>")?;
            send(&json!({ "type": "remove", "payload": name }))?;
            println!("✓ removed {name}");
            Ok(())
        }
        Some("function") => cmd_function(&args[1..]),
        _ => {
            print_usage();
            Ok(())
        }
    }
}

fn cmd_dev() -> Result<()> {
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

fn cmd_ls() -> Result<()> {
    let reply = send(&json!({ "type": "list" }))?;
    let Some(servers) = reply.get("payload").and_then(Value::as_array) else {
        println!("No servers registered. Run 'rack dev' in a project directory.");
        return Ok(());
    };
    if servers.is_empty() {
        println!("No servers registered. Run 'rack dev' in a project directory.");
        return Ok(());
    }

    let name_width = servers
        .iter()
        .filter_map(|server| server.get("name").and_then(Value::as_str))
        .map(str::len)
        .max()
        .unwrap_or(4);
    println!("{}", "─".repeat(name_width + 40));
    for server in servers {
        let name = server.get("name").and_then(Value::as_str).unwrap_or("");
        let url = server.get("url").and_then(Value::as_str).unwrap_or("");
        let running = server
            .get("running")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let dot = if running { "●" } else { "○" };
        println!("{dot}  {name:<name_width$}  {url}");
    }
    println!("{}", "─".repeat(name_width + 40));
    Ok(())
}

fn cmd_function(args: &[String]) -> Result<()> {
    match args.first().map(String::as_str) {
        Some("build") => cmd_function_build(args.get(1).map(String::as_str)),
        Some("install") => cmd_function_install(args.get(1).map(String::as_str)),
        _ => cmd_function_install(args.first().map(String::as_str)),
    }
}

fn cmd_function_build(path: Option<&str>) -> Result<()> {
    let source = function_source(path)?;
    let manifest_path = source.join("manifest.toml");
    let cargo_path = source.join("Cargo.toml");
    require_file(&manifest_path, "missing manifest.toml")?;
    require_file(&cargo_path, "missing Cargo.toml")?;

    let target_name = cargo_cdylib_target_name(&cargo_path)?;
    run_inherit(
        "cargo",
        &[
            "build",
            "--manifest-path",
            path_str(&cargo_path)?,
            "--release",
            "--target",
            "wasm32-wasip1",
        ],
        &source,
    )?;

    let wasm_name = format!("{}.wasm", target_name.replace('-', "_"));
    let built_wasm = source
        .join("target")
        .join("wasm32-wasip1")
        .join("release")
        .join(wasm_name);
    require_file(
        &built_wasm,
        &format!("cargo build did not produce {}", built_wasm.display()),
    )?;

    let output_wasm = source.join("functions.wasm");
    if output_wasm.exists() {
        fs::remove_file(&output_wasm).map_err(|error| error.to_string())?;
    }
    fs::copy(&built_wasm, &output_wasm).map_err(|error| error.to_string())?;
    println!("✓ built functions.wasm");
    println!("  {}", output_wasm.display());
    Ok(())
}

fn cmd_function_install(path: Option<&str>) -> Result<()> {
    let source = function_source(path)?;
    let manifest_path = source.join("manifest.toml");
    let wasm_path = source.join("functions.wasm");
    require_file(&manifest_path, "missing manifest.toml")?;
    require_file(&wasm_path, "missing functions.wasm")?;

    let manifest = fs::read_to_string(&manifest_path).map_err(|error| error.to_string())?;
    let name = manifest_name(&manifest).ok_or("manifest.toml must include name = \"...\"")?;
    if name.is_empty() {
        return Err("manifest.toml must include name = \"...\"".to_string());
    }

    let functions_dir = home_dir().join(".rack").join("functions");
    let destination = functions_dir.join(&name);
    if destination.exists() {
        return Err(format!("function '{name}' is already installed"));
    }

    fs::create_dir_all(&functions_dir).map_err(|error| error.to_string())?;
    fs::rename(&source, &destination).map_err(|error| error.to_string())?;
    symlink_dir(&destination, &source)?;

    println!("✓ installed {name}");
    println!("  {}", destination.display());
    println!("  {} -> {}", source.display(), destination.display());
    Ok(())
}

fn send(message: &Value) -> Result<Value> {
    let socket_path = home_dir().join(".config/rack/rack.sock");
    let mut stream = UnixStream::connect(&socket_path)
        .map_err(|_| "Rack.app is not running — open it first".to_string())?;
    stream
        .write_all(message.to_string().as_bytes())
        .and_then(|_| stream.write_all(b"\n"))
        .map_err(|error| error.to_string())?;

    let mut reply = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        let read = stream.read(&mut byte).map_err(|error| error.to_string())?;
        if read == 0 || byte[0] == b'\n' {
            break;
        }
        reply.push(byte[0]);
    }

    serde_json::from_slice(&reply).map_err(|error| error.to_string())
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

fn cargo_cdylib_target_name(manifest_path: &Path) -> Result<String> {
    let output = capture(
        "cargo",
        &[
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
            path_str(manifest_path)?,
        ],
        manifest_path.parent().unwrap_or_else(|| Path::new(".")),
    )?;
    let metadata = serde_json::from_str::<Value>(&output).map_err(|error| error.to_string())?;
    let package = metadata
        .get("packages")
        .and_then(Value::as_array)
        .and_then(|packages| packages.first())
        .ok_or("could not read cargo metadata")?;

    if let Some(targets) = package.get("targets").and_then(Value::as_array) {
        for target in targets {
            let has_cdylib_crate_type = target
                .get("crate_types")
                .and_then(Value::as_array)
                .is_some_and(|types| types.iter().any(|value| value.as_str() == Some("cdylib")));
            let has_cdylib_kind = target
                .get("kind")
                .and_then(Value::as_array)
                .is_some_and(|kinds| kinds.iter().any(|value| value.as_str() == Some("cdylib")));
            if has_cdylib_crate_type || has_cdylib_kind {
                if let Some(name) = target.get("name").and_then(Value::as_str) {
                    return Ok(name.to_string());
                }
            }
        }
    }

    package
        .get("name")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "Cargo.toml must include a package name".to_string())
}

fn function_source(path: Option<&str>) -> Result<PathBuf> {
    let source = match path {
        Some(path) => PathBuf::from(path),
        None => env::current_dir().map_err(|error| error.to_string())?,
    };
    let source = source.canonicalize().map_err(|error| error.to_string())?;
    if !source.is_dir() {
        return Err(format!(
            "function path is not a directory: {}",
            source.display()
        ));
    }
    Ok(source)
}

fn require_file(path: &Path, message: &str) -> Result<()> {
    if path.is_file() {
        Ok(())
    } else {
        Err(message.to_string())
    }
}

fn manifest_name(manifest: &str) -> Option<String> {
    for raw_line in manifest.lines() {
        let line = raw_line
            .split_once('#')
            .map_or(raw_line, |(line, _)| line)
            .trim();
        if !line.starts_with("name") {
            continue;
        }
        let (_, value) = line.split_once('=')?;
        let value = value.trim();
        return Some(
            value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
                .unwrap_or(value)
                .to_string(),
        );
    }
    None
}

fn home_dir() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn run_inherit(command: &str, args: &[&str], directory: &Path) -> Result<()> {
    let status = Command::new(command)
        .args(args)
        .current_dir(directory)
        .status()
        .map_err(|error| error.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{command} {} failed", args.join(" ")))
    }
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
    value.to_lowercase().replace(' ', "-")
}

fn path_str(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| format!("path is not valid UTF-8: {}", path.display()))
}

fn path_str_lossy(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(unix)]
fn symlink_dir(destination: &Path, source: &Path) -> Result<()> {
    std::os::unix::fs::symlink(destination, source).map_err(|error| error.to_string())
}

fn print_usage() {
    println!("rack — dev environment manager");
    println!();
    println!("  rack dev                    Register this directory with Rack.app");
    println!("  rack function build [path]  Build a Rust function package");
    println!("  rack function [path]        Install a local Rack function");
    println!("  rack ls                     List registered servers");
    println!("  rack start <name>           Start a server");
    println!("  rack stop <name>            Stop a server");
    println!("  rack rm <name>              Remove a server");
    println!();
    println!("Run 'rack dev' in a project directory. Rack.app must be running.");
}
