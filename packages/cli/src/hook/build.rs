use std::process::Command;

use anyhow::{bail, Context, Result};

pub fn run(path: &str) -> Result<()> {
    let status = Command::new("cargo")
        .args(["build", "--release", "--target", "wasm32-unknown-unknown"])
        .current_dir(path)
        .status()
        .with_context(|| format!("failed to run cargo for hook at {path}"))?;

    if !status.success() {
        bail!("hook build command failed for {path}");
    }

    println!("Built hook at {path}");
    Ok(())
}
