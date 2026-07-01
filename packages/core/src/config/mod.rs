mod backfill;
mod cache;
mod format;
mod parse;
mod paths;
mod preamble;
mod validate;
mod write;

pub use backfill::{backfill_str, load, BackfillError};
pub use parse::{parse_full_config, ParseError};
pub use paths::{cache_path, config_path};
use serde::{Deserialize, Serialize};
pub use validate::{validate_config, ValidationError, ValidationErrors};
pub use write::{
    add_service, remove_service, replace_service, save, save_at, set_terminal, WriteError,
};

pub const SUPPORTED_SCHEMA_VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    #[serde(skip)]
    pub schema_version: u8,
    pub use_standard_ports: bool,
    pub terminal: String,
    pub services: Vec<Service>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Service {
    pub id: String,
    pub name: String,
    pub host: String,
    pub run: String,
    pub working_dir: String,
    pub auto_start: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: SUPPORTED_SCHEMA_VERSION,
            use_standard_ports: false,
            terminal: "Ghostty".to_string(),
            services: Vec::new(),
        }
    }
}

impl Default for Service {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            host: String::new(),
            run: String::new(),
            working_dir: "~".to_string(),
            auto_start: false,
        }
    }
}
