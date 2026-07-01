use rack_core::config::{self, Service as ServiceConfig};

use super::RackRuntime;
use crate::supervisor::log::service_log_path;

impl RackRuntime {
    pub fn start_service(&self, id: &str) -> Result<(), String> {
        self.supervisor
            .start_service(id)
            .map_err(|error| error.to_string())?;
        self.refresh_after_command()
    }

    pub fn stop_service(&self, id: &str) -> Result<(), String> {
        self.supervisor
            .stop_service(id)
            .map_err(|error| error.to_string())?;
        self.refresh_after_command()
    }

    pub fn restart_service(&self, id: &str) -> Result<(), String> {
        self.supervisor
            .restart_service(id)
            .map_err(|error| error.to_string())?;
        self.refresh_after_command()
    }

    pub fn log(&self, id: &str) -> Result<String, String> {
        self.supervisor.log(id).map_err(|error| error.to_string())
    }

    pub fn log_path(&self, id: &str) -> Result<String, String> {
        if !self
            .configs
            .read()
            .map_err(|error| error.to_string())?
            .contains_key(id)
        {
            return Err(format!("unknown service: {id}"));
        }
        Ok(service_log_path(id).to_string_lossy().into_owned())
    }

    pub fn add_service(
        &mut self,
        service: ServiceConfig,
    ) -> Result<crate::snapshot::Snapshot, String> {
        let mut config = config::load().map_err(|error| error.to_string())?;
        config::add_service(&mut config, service.clone()).map_err(|error| error.to_string())?;
        self.supervisor
            .register(service.clone())
            .map_err(|error| error.to_string())?;
        config::save(&config).map_err(|error| error.to_string())?;
        self.configs
            .write()
            .map_err(|error| error.to_string())?
            .insert(service.id.clone(), service);
        self.snapshot()
    }

    pub fn edit_service(
        &mut self,
        id: &str,
        service: ServiceConfig,
    ) -> Result<crate::snapshot::Snapshot, String> {
        let mut config = config::load().map_err(|error| error.to_string())?;
        config::replace_service(&mut config, id, service.clone())
            .map_err(|error| error.to_string())?;
        self.supervisor
            .update(service.clone())
            .map_err(|error| error.to_string())?;
        config::save(&config).map_err(|error| error.to_string())?;
        self.configs
            .write()
            .map_err(|error| error.to_string())?
            .insert(service.id.clone(), service);
        self.snapshot()
    }

    pub fn remove_service(&mut self, id: &str) -> Result<crate::snapshot::Snapshot, String> {
        let mut config = config::load().map_err(|error| error.to_string())?;
        config::remove_service(&mut config, id).map_err(|error| error.to_string())?;
        self.supervisor
            .unregister(id)
            .map_err(|error| error.to_string())?;
        config::save(&config).map_err(|error| error.to_string())?;
        self.configs
            .write()
            .map_err(|error| error.to_string())?
            .remove(id);
        self.snapshot()
    }
}
