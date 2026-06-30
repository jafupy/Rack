use std::{fs, path::Path};

use super::{backfill::BackfillError, format::format_cached_config, paths::cache_path, Config};

pub fn cache_config(config: &Config) -> Result<(), BackfillError> {
    write_cache(&cache_path()?, config, Path::new("unknown"))
}

pub(crate) fn write_cache(
    path: &Path,
    config: &Config,
    source_path: &Path,
) -> Result<(), BackfillError> {
    let output = format_cached_config(config, source_path)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| BackfillError::CreateCacheDirectory {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    fs::write(path, output).map_err(|source| BackfillError::WriteCacheConfig {
        path: path.to_path_buf(),
        source,
    })
}
