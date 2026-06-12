//! Function logging helpers.
//!
//! Logs are written as structured JSON to stderr. Rack captures them and stores
//! daily JSONL files under `~/.rack/logs/functions/...`.

use serde_json::json;
use std::fmt;

/// Write an info-level function log line.
pub fn info(message: impl fmt::Display) {
    write("info", message);
}

/// Write a warning-level function log line.
pub fn warn(message: impl fmt::Display) {
    write("warn", message);
}

/// Write an error-level function log line.
pub fn error(message: impl fmt::Display) {
    write("error", message);
}

fn write(level: &str, message: impl fmt::Display) {
    eprintln!(
        "{}",
        json!({
            "rack_log": true,
            "level": level,
            "message": message.to_string(),
        })
    );
}
