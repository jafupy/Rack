use rack_core::config;

use super::RackRuntime;

impl RackRuntime {
    pub fn config_path() -> Result<String, String> {
        let path = config::config_path()
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "could not determine config path".to_string())?;
        Ok(path.to_string_lossy().into_owned())
    }

    pub fn terminal() -> Result<String, String> {
        Ok(config::load().map_err(|error| error.to_string())?.terminal)
    }

    pub fn set_terminal(terminal: &str) -> Result<(), String> {
        let mut config = config::load().map_err(|error| error.to_string())?;
        config::set_terminal(&mut config, terminal).map_err(|error| error.to_string())?;
        config::save(&config).map_err(|error| error.to_string())?;
        Ok(())
    }
}
