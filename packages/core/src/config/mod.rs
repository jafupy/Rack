mod backfill;
mod parse;
mod preamble;
mod validate;

pub use backfill::{
    backfill_at, backfill_str, cache_config, cache_config_at, cache_path, config_path, load,
    BackfillError,
};
pub use parse::{parse_full_config, ParseError};
use serde::{Deserialize, Serialize};
pub use validate::{validate_config, ValidationError, ValidationErrors};

pub const SUPPORTED_SCHEMA_VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Config {
    pub schema_version: u8,
    pub use_standard_ports: bool,
    pub terminal: String,
    pub services: Vec<Service>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Service {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub run: String,
    pub working_dir: String,
    pub auto_start: bool,
}
