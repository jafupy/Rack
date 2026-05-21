use chrono::{DateTime, Datelike, Local, NaiveTime, TimeZone, Weekday};
use std::collections::{BTreeMap, BTreeSet};
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
    id: String,
    path: String,
    method: String,
    function: String,
    wasm_path: PathBuf,
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

        package.routes.push(FunctionRoute {
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

fn find_route(method: &str, path: &str) -> Result<FunctionRoute, String> {
    let normalized = normalize_route_path(path);
    if normalized == "/" || normalized.starts_with("/_") {
        return Err("reserved rack.local path".to_string());
    }

    let mut matched: Option<FunctionRoute> = None;
    for package in load_functions() {
        if !package.errors.is_empty() {
            continue;
        }
        for route in package.routes {
            if route.method == method.to_uppercase() && route.path == normalized {
                if matched.is_some() {
                    return Err(format!(
                        "route conflict for {} {}",
                        method.to_uppercase(),
                        normalized
                    ));
                }
                matched = Some(route);
            }
        }
    }

    matched.ok_or_else(|| {
        format!(
            "no function route for {} {}",
            method.to_uppercase(),
            normalized
        )
    })
}

fn run_function(route: &FunctionRoute, request: &serde_json::Value) -> serde_json::Value {
    run_wasm_function(
        &route.function,
        &route.wasm_path,
        request,
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
    let Some(runtime) = ["/opt/homebrew/bin/wasmtime", "/usr/local/bin/wasmtime"]
        .iter()
        .find(|candidate| Path::new(candidate).is_file())
    else {
        return serde_json::json!({
            "type": response_type,
            "payload": {
                "status": 500,
                "headers": { "content-type": "text/plain" },
                "body": "rack: wasmtime is required to run functions.wasm"
            }
        });
    };

    let mut child = match Command::new(runtime)
        .arg("run")
        .arg("--invoke")
        .arg(function)
        .arg(wasm_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
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

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
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
    serde_json::json!({
        "type": response_type,
        "payload": {
            "status": 200,
            "headers": { "content-type": "text/plain" },
            "body": body
        }
    })
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
    let Some(core) = guard.as_ref() else {
        return c_string(r#"{"type":"error","message":"rack core is not running"}"#.to_string());
    };

    let parsed: serde_json::Value =
        serde_json::from_str(command).unwrap_or(serde_json::Value::Null);
    let command_type = parsed
        .get("type")
        .and_then(|value| value.as_str())
        .unwrap_or("");

    if command_type == "state.snapshot" {
        return c_string(format!(
            r#"{{"type":"state.snapshot","payload":{{"backend":"rust","started_at_ms":{},"servers":[],"functions":{}}}}}"#,
            core.started_at_ms,
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
