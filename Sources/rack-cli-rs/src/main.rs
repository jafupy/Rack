use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs as unix_fs;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

type Result<T> = std::result::Result<T, String>;

#[derive(serde::Deserialize)]
struct FunctionManifest {
    name: String,
    version: String,
    #[serde(default)]
    route: BTreeMap<String, ManifestRoute>,
    #[serde(default)]
    cron: BTreeMap<String, ManifestCron>,
}

#[derive(serde::Deserialize)]
struct ManifestRoute {
    path: String,
    method: String,
    function: String,
}

#[derive(serde::Deserialize)]
struct ManifestCron {
    schedule: String,
    function: String,
}

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
        Some("fn") => cmd_function(&args[1..]),
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
        Some("add") => {
            let path = args
                .iter()
                .skip(1)
                .find(|arg| !matches!(arg.as_str(), "--replace"))
                .map(String::as_str);
            cmd_function_add(path)
        }
        Some("build" | "compile") => cmd_function_build(args.get(1).map(String::as_str)),
        Some("init") => cmd_function_init(args.get(1).map(String::as_str)),
        Some("test") => cmd_function_test(&args[1..]),
        Some("install") => {
            let replace = args.iter().any(|arg| arg == "--replace");
            let link = args.iter().any(|arg| arg == "--link");
            let path = args
                .iter()
                .skip(1)
                .find(|arg| !matches!(arg.as_str(), "--replace" | "--link"))
                .map(String::as_str);
            cmd_function_install(path, replace, link)
        }
        Some("ls" | "list") => cmd_function_ls(),
        Some("rm" | "remove" | "uninstall") => {
            let name = args.get(1).ok_or("Usage: rack fn rm <name>")?;
            cmd_function_remove(name)
        }
        _ => cmd_function_install(args.first().map(String::as_str), false, false),
    }
}

fn cmd_function_build(path: Option<&str>) -> Result<()> {
    let source = function_source(path)?;
    let manifest_path = source.join("manifest.toml");
    let cargo_path = source.join("Cargo.toml");
    require_file(&manifest_path, "missing manifest.toml")?;
    require_file(&cargo_path, "missing Cargo.toml")?;

    ensure_sdk_installed()?;
    ensure_wasi_target()?;
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

fn cmd_function_add(path: Option<&str>) -> Result<()> {
    let source = function_source(path)?;
    cmd_function_build(Some(path_str(&source)?))?;
    cmd_function_install(Some(path_str(&source)?), true, false)
}

fn cmd_function_init(path: Option<&str>) -> Result<()> {
    let directory = match path {
        Some(path) => PathBuf::from(path),
        None => env::current_dir().map_err(|error| error.to_string())?,
    };
    let name = directory
        .file_name()
        .and_then(OsStr::to_str)
        .map(sanitize)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "rack-function".to_string());

    if directory.exists() {
        if !directory.is_dir() {
            return Err(format!(
                "path exists and is not a directory: {}",
                directory.display()
            ));
        }
        let mut entries = fs::read_dir(&directory).map_err(|error| error.to_string())?;
        if entries.next().is_some() {
            return Err(format!("directory is not empty: {}", directory.display()));
        }
    }

    fs::create_dir_all(directory.join("src")).map_err(|error| error.to_string())?;
    write_new_file(
        &directory.join("Cargo.toml"),
        &format!(
            r#"[package]
name = "rack-{name}"
version = "0.1.0"
edition = "2021"
publish = false

[lib]
crate-type = ["cdylib"]

[dependencies]
rack = {{ path = "{}" }}

[profile.release]
lto = true
opt-level = "z"
strip = true

[workspace]
"#,
            home_dir().join(".rack").join("sdk").display()
        ),
    )?;
    write_new_file(
        &directory.join("manifest.toml"),
        &format!(
            r#"name = "{name}"
version = "0.1.0"

[route.hello]
path = "/hello"
method = "GET"
function = "hello"

[cron.heartbeat]
schedule = "every 5 minutes"
function = "heartbeat"
"#
        ),
    )?;
    write_new_file(
        &directory.join("src/lib.rs"),
        r##"#[rack::route]
fn hello(_: rack::Request) -> rack::Response {
    rack::response::ok().text("hello from Rack wasm")
}

#[rack::cron]
fn heartbeat(event: rack::CronEvent) -> rack::Response {
    rack::log::info(format!("heartbeat scheduled at {}", event.scheduled_at));
    rack::response::ok().text("heartbeat")
}
"##,
    )?;

    ensure_sdk_installed()?;
    ensure_wasi_target()?;
    println!("✓ initialized {}", directory.display());
    println!("  cd {} && rack fn add", directory.display());
    Ok(())
}

fn cmd_function_test(args: &[String]) -> Result<()> {
    let (path, function_override) = parse_function_test_args(args);
    let source = function_source(path.as_deref())?;
    cmd_function_build(Some(path_str(&source)?))?;

    let manifest = read_function_manifest(&source.join("manifest.toml"))?;
    let (function, request) = function_test_request(&manifest, function_override.as_deref())?;
    let wasm_path = source.join("functions.wasm");
    require_file(&wasm_path, "missing functions.wasm")?;

    let runtime = find_wasmtime().ok_or("wasmtime is required to test functions.wasm")?;
    let mut command = Command::new(runtime);
    command
        .arg("run")
        .arg("--dir")
        .arg("/::/")
        .args(wasmtime_full_wasi_args())
        .arg("--invoke")
        .arg(&function)
        .arg(&wasm_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().map_err(|error| error.to_string())?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(request.to_string().as_bytes())
            .map_err(|error| error.to_string())?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| error.to_string())?;

    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }
    print!("{}", String::from_utf8_lossy(&output.stdout));
    if !output.status.success() {
        return Err(format!("function '{function}' failed"));
    }

    Ok(())
}

fn cmd_function_install(path: Option<&str>, replace: bool, link: bool) -> Result<()> {
    let source = function_source(path)?;
    let manifest_path = source.join("manifest.toml");
    let wasm_path = source.join("functions.wasm");
    require_file(&manifest_path, "missing manifest.toml")?;
    require_file(&wasm_path, "missing functions.wasm")?;

    let manifest = read_function_manifest(&manifest_path)?;
    if manifest.name.trim().is_empty() {
        return Err("manifest.toml must include name = \"...\"".to_string());
    }

    let functions_dir = home_dir().join(".rack").join("functions");
    let destination = functions_dir.join(&manifest.name);
    if fs::symlink_metadata(&destination).is_ok() {
        if replace {
            remove_installed_function_path(&destination)?;
        } else {
            return Err(format!(
                "function '{}' is already installed; use --replace to reinstall",
                manifest.name
            ));
        }
    }

    fs::create_dir_all(&functions_dir).map_err(|error| error.to_string())?;
    if link {
        unix_fs::symlink(&source, &destination).map_err(|error| error.to_string())?;
    } else {
        copy_function_package(&source, &destination)?;
    }

    println!(
        "✓ installed {}{}",
        manifest.name,
        if link { " (linked)" } else { "" }
    );
    println!("  {}", destination.display());
    Ok(())
}

fn cmd_function_ls() -> Result<()> {
    let functions_dir = home_dir().join(".rack").join("functions");
    let Ok(entries) = fs::read_dir(&functions_dir) else {
        println!("No functions installed.");
        return Ok(());
    };

    let mut rows = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = path.join("manifest.toml");
        match read_function_manifest(&manifest_path) {
            Ok(manifest) => rows.push((manifest, path, None)),
            Err(error) => rows.push((
                FunctionManifest {
                    name: path
                        .file_name()
                        .and_then(OsStr::to_str)
                        .unwrap_or("unknown")
                        .to_string(),
                    version: "?".to_string(),
                    route: BTreeMap::new(),
                    cron: BTreeMap::new(),
                },
                path,
                Some(error),
            )),
        }
    }

    rows.sort_by(|left, right| left.0.name.cmp(&right.0.name));
    if rows.is_empty() {
        println!("No functions installed.");
        return Ok(());
    }

    let name_width = rows
        .iter()
        .map(|(manifest, _, _)| manifest.name.len())
        .max()
        .unwrap_or(4);

    for (manifest, path, error) in rows {
        println!(
            "{:<name_width$}  {}  {}",
            manifest.name,
            manifest.version,
            path.display()
        );
        if let Some(error) = error {
            println!("  ! {error}");
            continue;
        }
        for (id, route) in manifest.route {
            println!(
                "  route {id}: {} {} -> {}",
                route.method.to_uppercase(),
                normalize_route_path(&route.path),
                route.function
            );
        }
        for (id, cron) in manifest.cron {
            println!("  cron  {id}: {} -> {}", cron.schedule, cron.function);
        }
    }

    Ok(())
}

fn cmd_function_remove(name: &str) -> Result<()> {
    let destination = home_dir().join(".rack").join("functions").join(name);
    if fs::symlink_metadata(&destination).is_err() {
        return Err(format!("function '{name}' is not installed"));
    }
    remove_installed_function_path(&destination)?;
    println!("✓ removed {name}");
    Ok(())
}

fn parse_function_test_args(args: &[String]) -> (Option<String>, Option<String>) {
    match args {
        [] => (None, None),
        [first] if Path::new(first).is_dir() => (Some(first.clone()), None),
        [first] => (None, Some(first.clone())),
        [first, second, ..] => (Some(first.clone()), Some(second.clone())),
    }
}

fn function_test_request(
    manifest: &FunctionManifest,
    function_override: Option<&str>,
) -> Result<(String, Value)> {
    if let Some(function) = function_override {
        return Ok((
            function.to_string(),
            json!({
                "method": "GET",
                "path": "/",
                "uri": "/",
                "headers": {},
                "body": "",
            }),
        ));
    }

    if let Some((_, route)) = manifest.route.iter().next() {
        let path = normalize_route_path(&route.path);
        return Ok((
            route.function.clone(),
            json!({
                "method": route.method.to_uppercase(),
                "path": path,
                "uri": path,
                "headers": {},
                "body": "",
            }),
        ));
    }

    if let Some((id, cron)) = manifest.cron.iter().next() {
        return Ok((
            cron.function.clone(),
            json!({
                "type": "schedule",
                "package": manifest.name,
                "id": id,
                "schedule": cron.schedule,
                "scheduled_at": "1970-01-01T00:00:00+00:00",
            }),
        ));
    }

    Err("manifest has no routes or crons to test".to_string())
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

fn read_function_manifest(path: &Path) -> Result<FunctionManifest> {
    let source = fs::read_to_string(path).map_err(|error| error.to_string())?;
    toml::from_str(&source).map_err(|error| format!("invalid manifest.toml: {error}"))
}

fn write_new_file(path: &Path, content: &str) -> Result<()> {
    if path.exists() {
        return Err(format!("file already exists: {}", path.display()));
    }
    fs::write(path, content).map_err(|error| error.to_string())
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

fn ensure_wasi_target() -> Result<()> {
    let installed = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match installed {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.lines().any(|line| line.trim() == "wasm32-wasip1") {
                return Ok(());
            }
        }
        Ok(_) | Err(_) => return Ok(()),
    }

    println!("rack: installing Rust target wasm32-wasip1");
    run_inherit(
        "rustup",
        &["target", "add", "wasm32-wasip1"],
        Path::new("."),
    )
}

fn ensure_sdk_installed() -> Result<()> {
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("could not locate rack SDK source")?
        .to_path_buf();
    let rack_source = source_root.join("rack-sdk-rs");
    let macros_source = source_root.join("rack-macros");
    require_file(&rack_source.join("Cargo.toml"), "missing rack SDK source")?;
    require_file(
        &macros_source.join("Cargo.toml"),
        "missing rack macro SDK source",
    )?;

    let destination = home_dir().join(".rack").join("sdk");
    if destination.exists() {
        fs::remove_dir_all(&destination).map_err(|error| error.to_string())?;
    }

    copy_dir_all(&rack_source, &destination)?;
    fs::remove_dir_all(destination.join("target")).ok();
    copy_dir_all(&macros_source, &destination.join("rack-macros"))?;

    let cargo_toml = destination.join("Cargo.toml");
    let manifest = fs::read_to_string(&cargo_toml).map_err(|error| error.to_string())?;
    let manifest = manifest.replace(
        r#"rack-macros = { path = "../rack-macros" }"#,
        r#"rack-macros = { path = "rack-macros" }"#,
    );
    fs::write(cargo_toml, manifest).map_err(|error| error.to_string())?;
    Ok(())
}

fn find_wasmtime() -> Option<PathBuf> {
    env::var_os("PATH")
        .into_iter()
        .flat_map(|path| env::split_paths(&path).collect::<Vec<_>>())
        .chain([
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/usr/local/bin"),
        ])
        .map(|dir| dir.join("wasmtime"))
        .find(|candidate| candidate.is_file())
}

fn wasmtime_full_wasi_args() -> Vec<OsString> {
    let mut args = vec![
        OsString::from("-S"),
        OsString::from("cli=y"),
        OsString::from("-S"),
        OsString::from("allow-ip-name-lookup=y"),
        OsString::from("-S"),
        OsString::from("tcp=y"),
        OsString::from("-S"),
        OsString::from("udp=y"),
        OsString::from("-S"),
        OsString::from("inherit-env=y"),
    ];

    for (key, value) in env::vars_os() {
        let mut env = key;
        env.push("=");
        env.push(value);
        args.push(OsString::from("--env"));
        args.push(env);
    }

    args
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

fn path_str(path: &Path) -> Result<&str> {
    path.to_str()
        .ok_or_else(|| format!("path is not valid UTF-8: {}", path.display()))
}

fn path_str_lossy(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn copy_dir_all(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    for entry in fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry.file_type().map_err(|error| error.to_string())?;

        if file_type.is_dir() {
            if entry.file_name() == "target" {
                continue;
            }
            copy_dir_all(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            fs::copy(&source_path, &destination_path).map_err(|error| error.to_string())?;
        }
    }

    Ok(())
}

#[cfg(unix)]
fn copy_function_package(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    fs::copy(
        source.join("manifest.toml"),
        destination.join("manifest.toml"),
    )
    .map_err(|error| error.to_string())?;
    fs::copy(
        source.join("functions.wasm"),
        destination.join("functions.wasm"),
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

fn remove_installed_function_path(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if metadata.file_type().is_symlink() || metadata.is_file() {
        fs::remove_file(path).map_err(|error| error.to_string())
    } else {
        fs::remove_dir_all(path).map_err(|error| error.to_string())
    }
}

fn normalize_route_path(path: &str) -> String {
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

fn print_usage() {
    println!("rack — dev environment manager");
    println!();
    println!("  rack dev                    Register this directory with Rack.app");
    println!("  rack fn add [path]          Build and install a Rust function package");
    println!("  rack fn init [path]         Create a Rust/WASI function package");
    println!("  rack fn compile [path]      Build a Rust function package");
    println!("  rack fn test [path] [fn]    Compile and run a function locally");
    println!("  rack fn install [path] [--replace] [--link]");
    println!("  rack fn ls                  List installed functions");
    println!("  rack fn rm <name>           Remove an installed function");
    println!("  rack ls                     List registered servers");
    println!("  rack start <name>           Start a server");
    println!("  rack stop <name>            Stop a server");
    println!("  rack rm <name>              Remove a server");
    println!();
    println!("Run 'rack dev' in a project directory. Rack.app must be running.");
}
