use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};

pub fn built_wasm_path(path: &Path) -> Result<PathBuf> {
    let release_dir = path.join("target/wasm32-unknown-unknown/release");
    let mut matches = fs::read_dir(&release_dir)
        .with_context(|| format!("missing build output directory {}", release_dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "wasm")
        })
        .collect::<Vec<_>>();
    matches.sort();

    match matches.as_slice() {
        [path] => Ok(path.clone()),
        [] => bail!("no wasm artifact found in {}", release_dir.display()),
        _ => bail!(
            "multiple wasm artifacts found in {}; cannot choose",
            release_dir.display()
        ),
    }
}
