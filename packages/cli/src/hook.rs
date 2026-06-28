use std::{
    fs,
    path::{Component, Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Context, Result};
use rack_hooks::HookRegistry;
use rack_services::hooks::HookSummary;

pub fn init(path: &str) -> Result<()> {
    let path = Path::new(path);
    if path.exists() {
        bail!("hook path already exists: {}", path.display());
    }

    fs::create_dir_all(path.join("src"))?;
    fs::write(path.join("Cargo.toml"), cargo_toml(path)?)?;
    fs::write(path.join("src/lib.rs"), sample_hook())?;

    println!("Initialized hook at {}", path.display());
    Ok(())
}

pub fn build(path: &str) -> Result<()> {
    let status = Command::new("cargo")
        .args(["build", "--release", "--target", "wasm32-unknown-unknown"])
        .current_dir(path)
        .status()
        .with_context(|| format!("failed to build hook at {path}"))?;

    if !status.success() {
        bail!("hook build failed");
    }

    println!("Built hook at {path}");
    Ok(())
}

pub fn list() -> Result<()> {
    let summaries = rack_services::hooks::load_deployed(&HookRegistry::default()).summaries;

    if summaries.is_empty() {
        println!("No hooks deployed");
        return Ok(());
    }

    for summary in summaries {
        print_hook_summary(&summary);
    }

    Ok(())
}

pub fn deploy(path: &str) -> Result<()> {
    build(path)?;

    let source = Path::new(path).canonicalize()?;
    let name = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("invalid hook path"))?;
    let destination = hooks_dir()?.join(name);

    if destination.exists() {
        bail!("deployed hook already exists: {}", destination.display());
    }

    fs::create_dir_all(destination.parent().expect("destination has parent"))?;
    fs::rename(&source, &destination)?;
    symlink_dir(&destination, &source)?;

    println!("Deployed hook to {}", destination.display());
    Ok(())
}

pub fn remove(name: &str) -> Result<()> {
    ensure_hook_name(name)?;
    let destination = hooks_dir()?.join(name);

    let metadata = fs::symlink_metadata(&destination)
        .with_context(|| format!("deployed hook not found: {}", destination.display()))?;
    let file_type = metadata.file_type();
    if file_type.is_symlink() || file_type.is_file() {
        fs::remove_file(&destination)?;
    } else if metadata.is_dir() {
        fs::remove_dir_all(&destination)?;
    } else {
        bail!(
            "deployed hook path is not removable: {}",
            destination.display()
        );
    }

    println!("Removed hook `{name}` from {}", hooks_dir()?.display());
    Ok(())
}

fn hooks_dir() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").ok_or_else(|| anyhow::anyhow!("HOME is not set"))?;
    Ok(PathBuf::from(home).join(".rack/hooks"))
}

fn ensure_hook_name(name: &str) -> Result<()> {
    let mut components = Path::new(name).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => Ok(()),
        _ => bail!("hook name must be a deployed hook directory name"),
    }
}

fn print_hook_summary(summary: &HookSummary) {
    println!("{}", summary.name);

    for route in &summary.routes {
        println!("  route\t{}\t{}", route.method, route.path);
    }

    for cron in &summary.crons {
        println!("  cron\t{}\t{}", cron.schedule, cron.hook);
    }

    for error in &summary.errors {
        println!("  error\t{error}");
    }
}

fn cargo_toml(path: &Path) -> Result<String> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("rack-hook")
        .replace('_', "-");
    let sdk_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("cli crate has packages parent")
        .join("sdk")
        .canonicalize()?;

    Ok(format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
rack = {{ path = "{}" }}
"#,
        sdk_path.display()
    ))
}

fn sample_hook() -> &'static str {
    r#"use rack::{Request, Response};

#[rack::route(GET, "hello")]
fn hello(_request: Request) -> Response {
    Response::text("hello from rack")
}
"#
}

#[cfg(unix)]
fn symlink_dir(destination: &Path, link: &Path) -> Result<()> {
    std::os::unix::fs::symlink(destination, link)?;
    Ok(())
}

#[cfg(windows)]
fn symlink_dir(destination: &Path, link: &Path) -> Result<()> {
    std::os::windows::fs::symlink_dir(destination, link)?;
    Ok(())
}
