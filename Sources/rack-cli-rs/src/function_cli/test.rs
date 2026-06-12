use super::build::cmd_function_build;
use super::support::{function_source, path_str, require_file};
use super::types::{normalize_route_path, read_function_manifest, FunctionManifest};
use crate::Result;
use serde_json::{json, Value};
use std::env;
use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub(crate) fn cmd_function_test(args: &[String]) -> Result<()> {
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
