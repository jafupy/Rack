use std::fs;

use anyhow::{bail, Context, Result};

use super::common::{ensure_hook_name, hooks_dir};

pub fn run(name: &str) -> Result<()> {
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
