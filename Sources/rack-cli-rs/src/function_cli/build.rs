use super::install::cmd_function_install;
use super::support::{
    capture, copy_dir_all, function_source, home_dir, path_str, require_file, run_inherit,
};
use crate::Result;
use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub(crate) fn cmd_function_build(path: Option<&str>) -> Result<()> {
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

pub(crate) fn cmd_function_add(path: Option<&str>) -> Result<()> {
    let source = function_source(path)?;
    cmd_function_build(Some(path_str(&source)?))?;
    cmd_function_install(Some(path_str(&source)?), true, false)
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

pub(crate) fn ensure_wasi_target() -> Result<()> {
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

pub(crate) fn ensure_sdk_installed() -> Result<()> {
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
