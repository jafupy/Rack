use super::build::{ensure_sdk_installed, ensure_wasi_target};
use super::support::{home_dir, sanitize, write_new_file};
use crate::Result;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;

pub(crate) fn cmd_function_init(path: Option<&str>) -> Result<()> {
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
