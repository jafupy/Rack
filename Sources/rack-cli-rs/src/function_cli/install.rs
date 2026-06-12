use super::support::{function_source, home_dir, require_file};
use super::types::{normalize_route_path, read_function_manifest, FunctionManifest};
use crate::Result;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::Path;

pub(crate) fn cmd_function_install(path: Option<&str>, replace: bool, link: bool) -> Result<()> {
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

pub(crate) fn cmd_function_ls() -> Result<()> {
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

pub(crate) fn cmd_function_remove(name: &str) -> Result<()> {
    let destination = home_dir().join(".rack").join("functions").join(name);
    if fs::symlink_metadata(&destination).is_err() {
        return Err(format!("function '{name}' is not installed"));
    }
    remove_installed_function_path(&destination)?;
    println!("✓ removed {name}");
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
