use std::path::Path;

use super::{backfill::BackfillError, preamble, Config};

pub(crate) fn format_config(config: &Config) -> Result<String, BackfillError> {
    Ok(format!(
        "{}\n\n{}",
        preamble::format_schema_header(config.schema_version),
        toml(config)?
    ))
}

pub(crate) fn format_cached_config(
    config: &Config,
    source_path: &Path,
) -> Result<String, BackfillError> {
    Ok(format!(
        "{}\n\n{}",
        preamble::format_generated_preamble(source_path, config.schema_version),
        toml(config)?
    ))
}

fn toml(config: &Config) -> Result<String, BackfillError> {
    Ok(toml::to_string_pretty(config)?)
}
