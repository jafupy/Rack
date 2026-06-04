use chrono::{DateTime, Datelike, Local, NaiveTime, TimeZone, Weekday};
use globset::GlobBuilder;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::ffi::{CStr, CString};
use std::io::{Read, Write};
use std::os::raw::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

type EventCallback = extern "C" fn(*const c_char, *mut c_void);

struct CoreState {
    started_at_ms: u128,
    callback: Option<EventCallback>,
    callback_context: usize,
    scheduler_stop: Arc<AtomicBool>,
}

static STATE: OnceLock<Mutex<Option<CoreState>>> = OnceLock::new();

fn state() -> &'static Mutex<Option<CoreState>> {
    STATE.get_or_init(|| Mutex::new(None))
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn c_string(value: String) -> *mut c_char {
    CString::new(value)
        .unwrap_or_else(|_| CString::new(r#"{"type":"error","message":"invalid string"}"#).unwrap())
        .into_raw()
}

#[derive(Clone, Debug)]
struct FunctionRoute {
    package: String,
    id: String,
    path: String,
    method: String,
    function: String,
    wasm_path: PathBuf,
}

#[derive(Clone, Debug)]
struct FunctionRouteMatch {
    route: FunctionRoute,
    request_path: String,
}

#[derive(Clone, Debug)]
struct FunctionCron {
    package: String,
    id: String,
    schedule: String,
    function: String,
    wasm_path: PathBuf,
}

#[derive(Clone, Debug)]
struct FunctionPackage {
    name: String,
    version: String,
    root: PathBuf,
    routes: Vec<FunctionRoute>,
    crons: Vec<FunctionCron>,
    errors: Vec<String>,
}

#[derive(serde::Deserialize)]
struct Manifest {
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

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn functions_dir() -> PathBuf {
    home_dir().join(".rack").join("functions")
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

fn parse_manifest(root: &Path) -> FunctionPackage {
    let manifest_path = root.join("manifest.toml");
    let wasm_path = root.join("functions.wasm");
    let mut package = FunctionPackage {
        name: root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string(),
        version: "0.0.0".to_string(),
        root: root.to_path_buf(),
        routes: Vec::new(),
        crons: Vec::new(),
        errors: Vec::new(),
    };

    if !wasm_path.is_file() {
        package.errors.push("missing functions.wasm".to_string());
    }

    let source = match std::fs::read_to_string(&manifest_path) {
        Ok(source) => source,
        Err(_) => {
            package.errors.push("missing manifest.toml".to_string());
            return package;
        }
    };

    let manifest: Manifest = match toml::from_str(&source) {
        Ok(manifest) => manifest,
        Err(error) => {
            package
                .errors
                .push(format!("invalid manifest.toml: {error}"));
            return package;
        }
    };

    package.name = manifest.name;
    package.version = manifest.version;

    for (id, route) in manifest.route {
        let normalized = normalize_route_path(&route.path);
        if normalized == "/" || normalized.starts_with("/_") {
            package
                .errors
                .push(format!("route '{id}' uses reserved path '{normalized}'"));
            continue;
        }
        if let Err(message) = validate_route_path(&normalized) {
            package.errors.push(format!("route '{id}' {message}"));
            continue;
        }

        package.routes.push(FunctionRoute {
            package: package.name.clone(),
            id,
            path: normalized,
            method: route.method.to_uppercase(),
            function: route.function,
            wasm_path: wasm_path.clone(),
        });
    }

    for (id, cron) in manifest.cron {
        package.crons.push(FunctionCron {
            package: package.name.clone(),
            id,
            schedule: cron.schedule,
            function: cron.function,
            wasm_path: wasm_path.clone(),
        });
    }

    if package.routes.is_empty() && package.crons.is_empty() {
        package
            .errors
            .push("manifest has no routes or crons".to_string());
    }

    package
}

fn load_functions() -> Vec<FunctionPackage> {
    let dir = functions_dir();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut packages: Vec<_> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .map(|path| parse_manifest(&path))
        .collect();
    packages.sort_by(|left, right| left.name.cmp(&right.name));
    packages
}

fn function_snapshot_json() -> serde_json::Value {
    let packages = load_functions();
    let mut claimed_routes: Vec<(String, String, String)> = Vec::new();
    let functions: Vec<_> = packages
        .into_iter()
        .map(|mut package| {
            for route in &package.routes {
                let key = (
                    route.method.clone(),
                    route.path.clone(),
                    package.name.clone(),
                );
                if claimed_routes
                    .iter()
                    .any(|(method, path, _)| method == &route.method && path == &route.path)
                {
                    package.errors.push(format!(
                        "route conflict for {} {}",
                        route.method, route.path
                    ));
                } else {
                    claimed_routes.push(key);
                }
            }
            serde_json::json!({
                "name": package.name,
                "version": package.version,
                "root": package.root,
                "routes": package.routes.iter().map(|route| serde_json::json!({
                    "id": route.id,
                    "path": route.path,
                    "method": route.method,
                    "function": route.function,
                })).collect::<Vec<_>>(),
                "crons": package.crons.iter().map(|cron| serde_json::json!({
                    "id": cron.id,
                    "schedule": cron.schedule,
                    "function": cron.function,
                })).collect::<Vec<_>>(),
                "errors": package.errors,
            })
        })
        .collect();

    serde_json::json!(functions)
}

fn route_has_glob(path: &str) -> bool {
    path.bytes()
        .any(|byte| matches!(byte, b'*' | b'?' | b'[' | b']' | b'{' | b'}'))
}

fn route_specificity(path: &str) -> usize {
    let literal_count = path
        .chars()
        .filter(|character| !matches!(character, '*' | '?' | '[' | ']' | '{' | '}' | ',' | '!'))
        .count();
    if route_has_glob(path) {
        literal_count
    } else {
        literal_count + 10_000
    }
}

fn route_matches(route_path: &str, request_path: &str) -> Result<bool, String> {
    if !route_has_glob(route_path) {
        return Ok(route_path == request_path);
    }

    GlobBuilder::new(route_path)
        .literal_separator(true)
        .backslash_escape(true)
        .build()
        .map(|glob| glob.compile_matcher().is_match(request_path))
        .map_err(|error| format!("invalid route glob '{route_path}': {error}"))
}

fn validate_route_path(route_path: &str) -> Result<(), String> {
    if route_has_glob(route_path) {
        GlobBuilder::new(route_path)
            .literal_separator(true)
            .backslash_escape(true)
            .build()
            .map(|_| ())
            .map_err(|error| format!("uses invalid glob '{route_path}': {error}"))
    } else {
        Ok(())
    }
}

fn find_route(method: &str, path: &str) -> Result<FunctionRouteMatch, String> {
    let normalized = normalize_route_path(path);
    if normalized == "/" || normalized.starts_with("/_") {
        return Err("reserved rack.local path".to_string());
    }

    let mut matched: Option<FunctionRoute> = None;
    let mut matched_score = 0usize;
    for package in load_functions() {
        if !package.errors.is_empty() {
            continue;
        }
        for route in package.routes {
            if route.method != method.to_uppercase() {
                continue;
            }

            if route_matches(&route.path, &normalized)? {
                let score = route_specificity(&route.path);
                if matched.is_some() && score == matched_score {
                    return Err(format!(
                        "route conflict for {} {}",
                        method.to_uppercase(),
                        normalized
                    ));
                }
                if score > matched_score {
                    matched = Some(route);
                    matched_score = score;
                }
            }
        }
    }

    matched
        .map(|route| FunctionRouteMatch {
            route,
            request_path: normalized.clone(),
        })
        .ok_or_else(|| {
            format!(
                "no function route for {} {}",
                method.to_uppercase(),
                normalized
            )
        })
}

fn route_match_request(
    route_match: &FunctionRouteMatch,
    request: &serde_json::Value,
) -> serde_json::Value {
    let mut request = request.clone();
    let is_glob = route_has_glob(&route_match.route.path);
    let route = serde_json::json!({
        "package": route_match.route.package,
        "id": route_match.route.id,
        "path": route_match.route.path,
        "pattern": route_match.route.path,
        "method": route_match.route.method,
        "function": route_match.route.function,
        "is_glob": is_glob,
        "matched_path": route_match.request_path,
    });

    if let Some(object) = request.as_object_mut() {
        object.insert("route".to_string(), route);
    }
    request
}

fn run_function(
    route_match: &FunctionRouteMatch,
    request: &serde_json::Value,
) -> serde_json::Value {
    let request = route_match_request(route_match, request);
    run_wasm_function(
        &route_match.route.function,
        &route_match.route.wasm_path,
        &request,
        "function.response",
    )
}

fn run_cron(cron: &FunctionCron, scheduled_at: DateTime<Local>) -> serde_json::Value {
    let request = serde_json::json!({
        "type": "schedule",
        "package": cron.package,
        "id": cron.id,
        "schedule": cron.schedule,
        "scheduled_at": scheduled_at.to_rfc3339(),
    });
    run_wasm_function(&cron.function, &cron.wasm_path, &request, "cron.response")
}

fn run_wasm_function(
    function: &str,
    wasm_path: &Path,
    request: &serde_json::Value,
    response_type: &str,
) -> serde_json::Value {
    let log_target = function_log_target(request, response_type);
    let started = Instant::now();
    append_function_log(
        &log_target,
        serde_json::json!({
            "time": Local::now().to_rfc3339(),
            "event": "started",
            "function": function,
        }),
    );

    let Some(runtime) = find_wasmtime() else {
        append_function_log(
            &log_target,
            serde_json::json!({
                "time": Local::now().to_rfc3339(),
                "event": "finished",
                "function": function,
                "status": 500,
                "duration_ms": started.elapsed().as_millis(),
            }),
        );
        return serde_json::json!({
            "type": response_type,
            "payload": {
                "status": 500,
                "headers": { "content-type": "text/plain" },
                "body": "rack: wasmtime is required to run functions.wasm"
            }
        });
    };

    let mut command = Command::new(runtime);
    command
        .arg("run")
        .arg("--dir")
        .arg("/::/")
        .args(wasmtime_full_wasi_args())
        .arg("--invoke")
        .arg(function)
        .arg(wasm_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            append_function_log(
                &log_target,
                serde_json::json!({
                    "time": Local::now().to_rfc3339(),
                    "event": "finished",
                    "function": function,
                    "status": 500,
                    "duration_ms": started.elapsed().as_millis(),
                    "error": error.to_string(),
                }),
            );
            return serde_json::json!({
                "type": response_type,
                "payload": {
                    "status": 500,
                    "headers": { "content-type": "text/plain" },
                    "body": format!("rack: failed to launch wasmtime: {error}")
                }
            });
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(request.to_string().as_bytes());
    }

    let output = match wait_with_timeout(child, Duration::from_secs(30)) {
        Ok(output) => output,
        Err(message) => {
            append_function_log(
                &log_target,
                serde_json::json!({
                    "time": Local::now().to_rfc3339(),
                    "event": "finished",
                    "function": function,
                    "status": 500,
                    "duration_ms": started.elapsed().as_millis(),
                    "error": message,
                }),
            );
            return serde_json::json!({
                "type": response_type,
                "payload": {
                    "status": 500,
                    "headers": { "content-type": "text/plain" },
                    "body": format!("rack: function runtime failed: {message}")
                }
            });
        }
    };

    write_stderr_logs(&log_target, &output.stderr);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        append_function_log(
            &log_target,
            serde_json::json!({
                "time": Local::now().to_rfc3339(),
                "event": "finished",
                "function": function,
                "status": 500,
                "duration_ms": started.elapsed().as_millis(),
            }),
        );
        return serde_json::json!({
            "type": response_type,
            "payload": {
                "status": 500,
                "headers": { "content-type": "text/plain" },
                "body": format!("rack: function '{function}' failed\n{}", stderr.trim())
            }
        });
    }

    let body = String::from_utf8_lossy(&output.stdout).to_string();
    if response_type == "function.response" {
        if let Some(payload) = parse_function_response(&body) {
            let status = payload
                .get("status")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(200);
            append_function_log(
                &log_target,
                serde_json::json!({
                    "time": Local::now().to_rfc3339(),
                    "event": "finished",
                    "function": function,
                    "status": status,
                    "duration_ms": started.elapsed().as_millis(),
                }),
            );
            return serde_json::json!({
                "type": response_type,
                "payload": payload,
            });
        }
    }

    append_function_log(
        &log_target,
        serde_json::json!({
            "time": Local::now().to_rfc3339(),
            "event": "finished",
            "function": function,
            "status": 200,
            "duration_ms": started.elapsed().as_millis(),
        }),
    );
    serde_json::json!({
        "type": response_type,
        "payload": {
            "status": 200,
            "headers": { "content-type": "text/plain" },
            "body": body
        }
    })
}

struct FunctionLogTarget {
    package: String,
    kind: &'static str,
    id: String,
}

fn function_log_target(request: &serde_json::Value, response_type: &str) -> FunctionLogTarget {
    if response_type == "function.response" {
        let route = request.get("route");
        return FunctionLogTarget {
            package: route
                .and_then(|route| route.get("package"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            kind: "routes",
            id: route
                .and_then(|route| route.get("id"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
        };
    }

    FunctionLogTarget {
        package: request
            .get("package")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        kind: "crons",
        id: request
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
    }
}

fn append_function_log(target: &FunctionLogTarget, entry: serde_json::Value) {
    let date = Local::now().format("%Y-%m-%d").to_string();
    let path = home_dir()
        .join(".rack")
        .join("logs")
        .join("functions")
        .join(safe_log_component(&target.package))
        .join(target.kind)
        .join(safe_log_component(&target.id))
        .join(format!("{date}.jsonl"));

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{entry}");
    }
}

fn write_stderr_logs(target: &FunctionLogTarget, stderr: &[u8]) {
    let stderr = String::from_utf8_lossy(stderr);
    for line in stderr
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            if value
                .get("rack_log")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
            {
                append_function_log(
                    target,
                    serde_json::json!({
                        "time": Local::now().to_rfc3339(),
                        "level": value.get("level").and_then(serde_json::Value::as_str).unwrap_or("info"),
                        "message": value.get("message").and_then(serde_json::Value::as_str).unwrap_or(""),
                    }),
                );
                continue;
            }
        }

        append_function_log(
            target,
            serde_json::json!({
                "time": Local::now().to_rfc3339(),
                "level": "stderr",
                "message": line,
            }),
        );
    }
}

fn safe_log_component(value: &str) -> String {
    let component = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();

    if component.is_empty() {
        "unknown".to_string()
    } else {
        component
    }
}

fn find_wasmtime() -> Option<PathBuf> {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
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
    args.extend(wasi_env_args());
    args
}

fn parse_function_response(stdout: &str) -> Option<serde_json::Value> {
    let value: serde_json::Value = serde_json::from_str(stdout.trim()).ok()?;
    let status = value
        .get("status")
        .and_then(|value| value.as_u64())
        .filter(|status| (100..=599).contains(status))
        .unwrap_or(200);
    let headers = value
        .get("headers")
        .and_then(|headers| headers.as_object())
        .map(|headers| {
            headers
                .iter()
                .filter_map(|(key, value)| {
                    value.as_str().map(|value| {
                        (
                            key.to_ascii_lowercase(),
                            serde_json::Value::String(value.to_string()),
                        )
                    })
                })
                .collect::<serde_json::Map<_, _>>()
        })
        .unwrap_or_else(|| {
            let mut headers = serde_json::Map::new();
            headers.insert(
                "content-type".to_string(),
                serde_json::Value::String("text/plain".to_string()),
            );
            headers
        });
    let body = value
        .get("body")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();

    Some(serde_json::json!({
        "status": status,
        "headers": headers,
        "body": body,
    }))
}

fn wasi_env_args() -> Vec<OsString> {
    let mut args = Vec::new();
    for (key, value) in std::env::vars_os() {
        let mut env = key;
        env.push("=");
        env.push(value);
        args.push(OsString::from("--env"));
        args.push(env);
    }
    args
}

fn wait_with_timeout(mut child: std::process::Child, timeout: Duration) -> Result<Output, String> {
    let stdout_reader = child.stdout.take().map(|mut stdout| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = stdout.read_to_end(&mut bytes);
            bytes
        })
    });
    let stderr_reader = child.stderr.take().map(|mut stderr| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = stderr.read_to_end(&mut bytes);
            bytes
        })
    });

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.map(|reader| reader.join());
                let _ = stderr_reader.map(|reader| reader.join());
                return Err(format!("timed out after {} seconds", timeout.as_secs()));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(error) => return Err(error.to_string()),
        }
    };

    let stdout = stdout_reader
        .and_then(|reader| reader.join().ok())
        .unwrap_or_default();
    let stderr = stderr_reader
        .and_then(|reader| reader.join().ok())
        .unwrap_or_default();

    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

#[derive(Clone)]
enum Schedule {
    Interval(chrono::Duration),
    Calendar(CalendarSchedule),
}

#[derive(Clone)]
struct CalendarSchedule {
    days: CalendarDays,
    time: NaiveTime,
}

#[derive(Clone)]
enum CalendarDays {
    Any,
    Weekdays,
    One(Weekday),
}

fn parse_schedule(expression: &str) -> Result<Schedule, String> {
    let normalized = expression.trim().to_lowercase();
    if let Some(rest) = normalized.strip_prefix("every ") {
        return parse_interval(rest).map(Schedule::Interval);
    }

    parse_calendar(&normalized).map(Schedule::Calendar)
}

fn parse_interval(expression: &str) -> Result<chrono::Duration, String> {
    let mut parts = expression.split_whitespace();
    let amount = parts
        .next()
        .ok_or_else(|| "missing interval amount".to_string())?
        .parse::<f64>()
        .map_err(|_| "invalid interval amount".to_string())?;
    let unit = parts
        .next()
        .ok_or_else(|| "missing interval unit".to_string())?;

    if amount <= 0.0 {
        return Err("interval must be positive".to_string());
    }

    let seconds = match unit.trim_end_matches('s') {
        "second" => amount,
        "minute" => amount * 60.0,
        "hour" => amount * 60.0 * 60.0,
        "day" => amount * 60.0 * 60.0 * 24.0,
        _ => return Err(format!("unsupported interval unit '{unit}'")),
    };

    let seconds = seconds.round() as i64;
    if seconds < 1 {
        return Err("interval must be at least one second".to_string());
    }

    Ok(chrono::Duration::seconds(seconds))
}

fn parse_calendar(expression: &str) -> Result<CalendarSchedule, String> {
    let (days, time) = if let Some((day, time)) = expression.split_once(" at ") {
        (parse_days(day.trim())?, parse_time(time.trim())?)
    } else {
        (CalendarDays::Any, parse_time(expression)?)
    };

    Ok(CalendarSchedule { days, time })
}

fn parse_days(value: &str) -> Result<CalendarDays, String> {
    match value {
        "weekday" | "weekdays" => Ok(CalendarDays::Weekdays),
        "monday" | "mon" => Ok(CalendarDays::One(Weekday::Mon)),
        "tuesday" | "tue" | "tues" => Ok(CalendarDays::One(Weekday::Tue)),
        "wednesday" | "wed" => Ok(CalendarDays::One(Weekday::Wed)),
        "thursday" | "thu" | "thur" | "thurs" => Ok(CalendarDays::One(Weekday::Thu)),
        "friday" | "fri" => Ok(CalendarDays::One(Weekday::Fri)),
        "saturday" | "sat" => Ok(CalendarDays::One(Weekday::Sat)),
        "sunday" | "sun" => Ok(CalendarDays::One(Weekday::Sun)),
        _ => Err(format!("unsupported calendar day '{value}'")),
    }
}

fn parse_time(value: &str) -> Result<NaiveTime, String> {
    let compact = value.replace(' ', "");
    let (clock, suffix) = if let Some(clock) = compact.strip_suffix("am") {
        (clock, Some("am"))
    } else if let Some(clock) = compact.strip_suffix("pm") {
        (clock, Some("pm"))
    } else {
        (compact.as_str(), None)
    };

    let mut pieces = clock.split(':');
    let mut hour = pieces
        .next()
        .ok_or_else(|| "missing hour".to_string())?
        .parse::<u32>()
        .map_err(|_| "invalid hour".to_string())?;
    let minute = pieces
        .next()
        .map(|minute| {
            minute
                .parse::<u32>()
                .map_err(|_| "invalid minute".to_string())
        })
        .transpose()?
        .unwrap_or(0);

    match suffix {
        Some("am") if hour == 12 => hour = 0,
        Some("pm") if hour < 12 => hour += 12,
        _ => {}
    }

    NaiveTime::from_hms_opt(hour, minute, 0).ok_or_else(|| format!("invalid time '{value}'"))
}

fn next_after(schedule: &Schedule, after: DateTime<Local>) -> Option<DateTime<Local>> {
    match schedule {
        Schedule::Interval(interval) => Some(after + *interval),
        Schedule::Calendar(calendar) => next_calendar_after(calendar, after),
    }
}

fn next_calendar_after(
    schedule: &CalendarSchedule,
    after: DateTime<Local>,
) -> Option<DateTime<Local>> {
    for offset in 0..14 {
        let date = after.date_naive() + chrono::Duration::days(offset);
        let weekday = date.weekday();
        let day_matches = match schedule.days {
            CalendarDays::Any => true,
            CalendarDays::Weekdays => !matches!(weekday, Weekday::Sat | Weekday::Sun),
            CalendarDays::One(day) => weekday == day,
        };
        if !day_matches {
            continue;
        }

        let naive = date.and_time(schedule.time);
        let Some(candidate) = Local.from_local_datetime(&naive).single() else {
            continue;
        };
        if candidate > after {
            return Some(candidate);
        }
    }
    None
}

fn start_scheduler(stop: Arc<AtomicBool>, callback: Option<EventCallback>, context: usize) {
    std::thread::spawn(move || {
        let mut next_runs: BTreeMap<String, DateTime<Local>> = BTreeMap::new();
        let mut reported_invalid_schedules: BTreeSet<String> = BTreeSet::new();

        while !stop.load(Ordering::Relaxed) {
            let now = Local::now();
            for package in load_functions() {
                if !package.errors.is_empty() {
                    continue;
                }

                for cron in package.crons {
                    let key = format!("{}:{}", cron.package, cron.id);
                    let schedule = match parse_schedule(&cron.schedule) {
                        Ok(schedule) => {
                            reported_invalid_schedules.remove(&key);
                            schedule
                        }
                        Err(message) => {
                            if reported_invalid_schedules.insert(key.clone()) {
                                if let Some(callback) = callback {
                                    emit(
                                        callback,
                                        context,
                                        &serde_json::json!({
                                            "type": "cron.error",
                                            "payload": {
                                                "package": cron.package,
                                                "id": cron.id,
                                                "schedule": cron.schedule,
                                                "error": message,
                                            }
                                        })
                                        .to_string(),
                                    );
                                }
                            }
                            continue;
                        }
                    };

                    let due_at = *next_runs
                        .entry(key.clone())
                        .or_insert_with(|| next_after(&schedule, now).unwrap_or(now));
                    if due_at > now {
                        continue;
                    }

                    if let Some(callback) = callback {
                        emit(
                            callback,
                            context,
                            &serde_json::json!({
                                "type": "cron.started",
                                "payload": {
                                    "package": cron.package,
                                    "id": cron.id,
                                    "schedule": cron.schedule,
                                    "scheduled_at": due_at.to_rfc3339(),
                                }
                            })
                            .to_string(),
                        );
                    }

                    let result = run_cron(&cron, due_at);

                    if let Some(callback) = callback {
                        emit(
                            callback,
                            context,
                            &serde_json::json!({
                                "type": "cron.finished",
                                "payload": {
                                    "package": cron.package,
                                    "id": cron.id,
                                    "schedule": cron.schedule,
                                    "scheduled_at": due_at.to_rfc3339(),
                                    "result": result,
                                }
                            })
                            .to_string(),
                        );
                    }

                    if let Some(next) = next_after(&schedule, now) {
                        next_runs.insert(key, next);
                    }
                }
            }

            std::thread::sleep(Duration::from_secs(1));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_interval_schedules() {
        match parse_schedule("every 0.5 hours").unwrap() {
            Schedule::Interval(duration) => assert_eq!(duration.num_minutes(), 30),
            Schedule::Calendar(_) => panic!("expected interval"),
        }
    }

    #[test]
    fn parses_calendar_schedules() {
        let schedule = parse_schedule("sunday at 5pm").unwrap();
        match schedule {
            Schedule::Calendar(calendar) => {
                assert!(matches!(calendar.days, CalendarDays::One(Weekday::Sun)));
                assert_eq!(calendar.time, NaiveTime::from_hms_opt(17, 0, 0).unwrap());
            }
            Schedule::Interval(_) => panic!("expected calendar"),
        }
    }

    #[test]
    fn parses_structured_http_function_response() {
        let payload = parse_function_response(
            r#"{"status":201,"headers":{"Content-Type":"application/json"},"body":"{\"ok\":true}"}"#,
        )
        .unwrap();

        assert_eq!(payload["status"], 201);
        assert_eq!(payload["headers"]["content-type"], "application/json");
        assert_eq!(payload["body"], r#"{"ok":true}"#);
    }

    #[test]
    fn matches_recursive_glob_routes() {
        assert!(route_matches("/assets/**/*.js", "/assets/app/main.js").unwrap());
        assert!(!route_matches("/assets/**/*.js", "/assets/app/main.css").unwrap());
        assert!(!route_matches("/assets/*.js", "/assets/app/main.js").unwrap());
    }

    #[test]
    fn exact_routes_are_more_specific_than_globs() {
        assert!(route_specificity("/gcse") > route_specificity("/*"));
        assert!(route_specificity("/assets/images/*") > route_specificity("/assets/**"));
    }

    #[test]
    fn route_match_request_adds_route_metadata() {
        let route_match = FunctionRouteMatch {
            route: FunctionRoute {
                package: "pkg".to_string(),
                id: "assets".to_string(),
                path: "/assets/**".to_string(),
                method: "GET".to_string(),
                function: "serve".to_string(),
                wasm_path: PathBuf::from("functions.wasm"),
            },
            request_path: "/assets/app/main.js".to_string(),
        };
        let request = serde_json::json!({
            "method": "GET",
            "path": "/assets/app/main.js",
            "uri": "/assets/app/main.js?debug=1",
            "headers": {},
            "body": "",
        });

        let request = route_match_request(&route_match, &request);

        assert_eq!(request["route"]["package"], "pkg");
        assert_eq!(request["route"]["id"], "assets");
        assert_eq!(request["route"]["pattern"], "/assets/**");
        assert_eq!(request["route"]["matched_path"], "/assets/app/main.js");
        assert_eq!(request["route"]["is_glob"], true);
    }
}

fn emit(callback: EventCallback, context: usize, json: &str) {
    if let Ok(message) = CString::new(json) {
        callback(message.as_ptr(), context as *mut c_void);
    }
}

#[no_mangle]
pub extern "C" fn rack_core_start(
    config_json: *const c_char,
    callback: Option<EventCallback>,
    callback_context: *mut c_void,
) -> c_int {
    let config = unsafe {
        if config_json.is_null() {
            "{}"
        } else {
            CStr::from_ptr(config_json).to_str().unwrap_or("{}")
        }
    };

    let scheduler_stop = Arc::new(AtomicBool::new(false));
    start_scheduler(scheduler_stop.clone(), callback, callback_context as usize);

    let mut guard = state().lock().unwrap();
    *guard = Some(CoreState {
        started_at_ms: now_ms(),
        callback,
        callback_context: callback_context as usize,
        scheduler_stop,
    });

    if let Some(callback) = callback {
        emit(
            callback,
            callback_context as usize,
            &format!(
                r#"{{"type":"core.started","payload":{{"config":{},"backend":"rust"}}}}"#,
                config
            ),
        );
    }

    0
}

#[no_mangle]
pub extern "C" fn rack_core_stop() {
    let mut guard = state().lock().unwrap();
    let previous = guard.take();
    drop(guard);

    if let Some(core) = previous {
        core.scheduler_stop.store(true, Ordering::Relaxed);
        if let Some(callback) = core.callback {
            emit(
                callback,
                core.callback_context,
                r#"{"type":"core.stopped","payload":{"backend":"rust"}}"#,
            );
        }
    }
}

#[no_mangle]
pub extern "C" fn rack_core_command(command_json: *const c_char) -> *mut c_char {
    let command = unsafe {
        if command_json.is_null() {
            ""
        } else {
            CStr::from_ptr(command_json).to_str().unwrap_or("")
        }
    };

    let guard = state().lock().unwrap();
    let Some(started_at_ms) = guard.as_ref().map(|core| core.started_at_ms) else {
        return c_string(r#"{"type":"error","message":"rack core is not running"}"#.to_string());
    };
    drop(guard);

    let parsed: serde_json::Value =
        serde_json::from_str(command).unwrap_or(serde_json::Value::Null);
    let command_type = parsed
        .get("type")
        .and_then(|value| value.as_str())
        .unwrap_or("");

    if command_type == "state.snapshot" {
        return c_string(format!(
            r#"{{"type":"state.snapshot","payload":{{"backend":"rust","started_at_ms":{},"servers":[],"functions":{}}}}}"#,
            started_at_ms,
            function_snapshot_json()
        ));
    }

    if command_type == "function.http" {
        let payload = parsed.get("payload").unwrap_or(&serde_json::Value::Null);
        let method = payload
            .get("method")
            .and_then(|value| value.as_str())
            .unwrap_or("GET");
        let path = payload
            .get("path")
            .and_then(|value| value.as_str())
            .unwrap_or("/");
        match find_route(method, path) {
            Ok(route) => return c_string(run_function(&route, payload).to_string()),
            Err(message) => {
                return c_string(serde_json::json!({
                    "type": "function.response",
                    "payload": {
                        "status": if message.starts_with("no function route") { 404 } else { 409 },
                        "headers": { "content-type": "text/plain" },
                        "body": format!("rack: {message}")
                    }
                }).to_string());
            }
        }
    }

    c_string(format!(
        r#"{{"type":"ack","payload":{{"backend":"rust","command":{}}}}}"#,
        if command.is_empty() { "null" } else { command }
    ))
}

#[no_mangle]
pub extern "C" fn rack_core_free_string(value: *mut c_char) {
    if value.is_null() {
        return;
    }

    unsafe {
        drop(CString::from_raw(value));
    }
}
