use super::response::parse_function_response;
use super::timeout::wait_with_timeout;
use super::wasmtime::{find_wasmtime, wasmtime_full_wasi_args};
use crate::functions::logs::{append_function_log, function_log_target, write_stderr_logs};
use chrono::Local;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub(super) fn run_wasm_function(
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
