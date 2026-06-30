use std::{fs, path::Path};

use anyhow::{bail, Result};

use super::{
    build,
    common::{hooks_dir, symlink_dir},
};

pub fn run(path: &str) -> Result<()> {
    build::run(path)?;

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
