use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use anyhow::{bail, Result};

pub fn hooks_dir() -> Result<PathBuf> {
    Ok(rack_dir()?.join("hooks"))
}

pub fn sdk_path() -> Result<PathBuf> {
    Ok(rack_dir()?.join("sdk"))
}

pub fn ensure_hook_name(name: &str) -> Result<()> {
    let mut components = Path::new(name).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => Ok(()),
        _ => bail!("hook name must be a deployed hook directory name"),
    }
}

pub fn install_sdk() -> Result<()> {
    let rack = rack_dir()?;
    fs::create_dir_all(&rack)?;
    copy_package(&package_path("sdk")?, &rack.join("sdk"))?;
    copy_package(&package_path("sdk-macro")?, &rack.join("sdk-macro"))?;
    Ok(())
}

fn rack_dir() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").ok_or_else(|| anyhow::anyhow!("HOME is not set"))?;
    Ok(PathBuf::from(home).join(".rack"))
}

fn package_path(name: &str) -> Result<PathBuf> {
    Ok(Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("cli crate has packages parent")
        .join(name)
        .canonicalize()?)
}

fn copy_package(source: &Path, destination: &Path) -> Result<()> {
    if destination.exists() {
        fs::remove_dir_all(destination)?;
    }
    copy_dir(source, destination)
}

fn copy_dir(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        let target = destination.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &target)?;
        } else if path.is_file() {
            fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

pub fn symlink_dir(destination: &Path, link: &Path) -> Result<()> {
    std::os::unix::fs::symlink(destination, link)?;
    Ok(())
}
