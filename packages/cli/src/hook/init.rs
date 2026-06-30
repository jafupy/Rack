use std::{fs, path::Path};

use anyhow::{bail, Result};

use super::common::{install_sdk, sdk_path};

const CARGO_TEMPLATE: &str = include_str!("../../../hooks/public/Cargo.toml");
const LIB_TEMPLATE: &str = include_str!("../../../hooks/public/lib.rs");

pub fn run(path: &str) -> Result<()> {
    let path = Path::new(path);
    if path.exists() {
        bail!("hook path already exists: {}", path.display());
    }

    install_sdk()?;
    fs::create_dir_all(path.join("src"))?;
    fs::write(path.join("Cargo.toml"), cargo_toml(path)?)?;
    fs::write(path.join("src/lib.rs"), LIB_TEMPLATE)?;

    println!("Initialized hook at {}", path.display());
    Ok(())
}

fn cargo_toml(path: &Path) -> Result<String> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("rack-hook")
        .replace('_', "-");
    let sdk_path = sdk_path()?;

    Ok(CARGO_TEMPLATE
        .replace("{{name}}", &name)
        .replace("{{sdk_path}}", &sdk_path.display().to_string()))
}
