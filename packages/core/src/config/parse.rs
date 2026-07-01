use thiserror::Error;

use super::{
    preamble::{self, PreambleError},
    Config,
};

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("missing config schema header (expected `# RACK:V<version>`)")]
    MissingSchemaHeader,

    #[error("invalid config schema version in header `{0}`")]
    InvalidSchemaVersion(String),

    #[error("unsupported config schema version `{0}`")]
    UnsupportedSchemaVersion(u8),

    #[error("failed to parse TOML config: {0}")]
    Toml(#[from] toml::de::Error),
}

pub fn parse_full_config(input: &str) -> Result<Config, ParseError> {
    let schema_version = parse_schema_version(input)?;
    let mut config = parse_partial_config(input)?;
    config.schema_version = schema_version;

    Ok(config)
}

pub(crate) fn parse_partial_config(input: &str) -> Result<Config, ParseError> {
    Ok(toml::from_str(input)?)
}

pub(crate) fn parse_schema_version(input: &str) -> Result<u8, ParseError> {
    preamble::parse_schema_version(input).map_err(ParseError::from)
}

impl From<PreambleError> for ParseError {
    fn from(error: PreambleError) -> Self {
        match error {
            PreambleError::MissingSchemaHeader => Self::MissingSchemaHeader,
            PreambleError::InvalidSchemaVersion(version) => Self::InvalidSchemaVersion(version),
            PreambleError::UnsupportedSchemaVersion(version) => {
                Self::UnsupportedSchemaVersion(version)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_config() {
        let config = parse_full_config(include_str!("../../public/default-config.toml")).unwrap();

        assert_eq!(config.schema_version, 1);
        assert!(!config.use_standard_ports);
        assert_eq!(config.terminal, "Ghostty");
        assert!(config.services.is_empty());
    }

    #[test]
    fn rejects_missing_schema_header() {
        let error =
            parse_full_config("terminal = \"Ghostty\"\nuse_standard_ports = false").unwrap_err();

        assert!(matches!(error, ParseError::MissingSchemaHeader));
    }

    #[test]
    fn rejects_invalid_schema_header() {
        let error = parse_full_config("# RACK:Vlol\nterminal = \"Ghostty\"").unwrap_err();

        assert!(matches!(error, ParseError::InvalidSchemaVersion(_)));
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let error = parse_full_config("# RACK:V99\nterminal = \"Ghostty\"").unwrap_err();

        assert!(matches!(error, ParseError::UnsupportedSchemaVersion(99)));
    }

    #[test]
    fn rejects_unknown_top_level_fields() {
        let error = parse_full_config(
            r#"# RACK:V1

use_standard_ports = false
terminal = "Ghostty"
auto_statr = true
services = []
"#,
        )
        .unwrap_err();

        assert!(matches!(error, ParseError::Toml(_)));
    }

    #[test]
    fn rejects_unknown_service_fields() {
        let error = parse_full_config(
            r#"# RACK:V1

use_standard_ports = false
terminal = "Ghostty"

[[services]]
id = "A123C23D-DBCB-4689-8A7F-D888B8A47BAE"
name = "DEFAULT"
host = "default"
run = "echo hi"
working_dir = "~"
auto_statr = true
"#,
        )
        .unwrap_err();

        assert!(matches!(error, ParseError::Toml(_)));
    }
}
