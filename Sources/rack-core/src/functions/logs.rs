use chrono::Local;
use std::io::Write;
use std::path::PathBuf;

pub(super) struct FunctionLogTarget {
    package: String,
    kind: &'static str,
    id: String,
}

pub(super) fn function_log_target(
    request: &serde_json::Value,
    response_type: &str,
) -> FunctionLogTarget {
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

pub(super) fn append_function_log(target: &FunctionLogTarget, entry: serde_json::Value) {
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

pub(super) fn write_stderr_logs(target: &FunctionLogTarget, stderr: &[u8]) {
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

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}
