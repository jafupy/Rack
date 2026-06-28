use std::collections::HashMap;

use rack_core::config::Service as ServiceConfig;

use super::{RegistryError, Service, ServiceState, ServiceView};

#[derive(Debug, Default)]
pub struct Registry {
    services: HashMap<String, Service>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, config: ServiceConfig) -> Result<(), RegistryError> {
        self.ensure_can_insert(&config)?;
        self.services
            .insert(config.id.clone(), Service::new(config));
        Ok(())
    }

    pub fn update(&mut self, config: ServiceConfig) -> Result<(), RegistryError> {
        self.ensure_exists(&config.id)?;
        self.ensure_host_available(&config.id, &config.host)?;

        let service = self.service_mut(&config.id)?;
        service.config = config;
        Ok(())
    }

    pub fn unregister(&mut self, id: &str) -> Result<ServiceConfig, RegistryError> {
        self.services
            .remove(id)
            .map(|service| service.config)
            .ok_or_else(|| RegistryError::UnknownService(id.to_string()))
    }

    pub fn resolve(&self, id: &str) -> Option<&Service> {
        self.services.get(id)
    }

    pub fn resolve_mut(&mut self, id: &str) -> Option<&mut Service> {
        self.services.get_mut(id)
    }

    pub fn config(&self, id: &str) -> Result<ServiceConfig, RegistryError> {
        self.service(id).map(|service| service.config.clone())
    }

    pub fn status(&self, id: &str) -> Result<ServiceState, RegistryError> {
        self.service(id).map(|service| service.state.clone())
    }

    pub fn list(&self) -> Vec<ServiceView> {
        self.services.values().map(Service::view).collect()
    }

    pub fn mark_starting(&mut self, id: &str) -> Result<(), RegistryError> {
        let service = self.service_mut(id)?;

        if !matches!(service.state, ServiceState::Stopped) {
            return Err(RegistryError::AlreadyStarted(id.to_string()));
        }

        Ok(())
    }

    pub fn mark_spawned(&mut self, id: &str, pid: i32, pgid: i32) -> Result<(), RegistryError> {
        let service = self.service_mut(id)?;
        service.state = ServiceState::Starting { pid, pgid };
        Ok(())
    }

    pub fn mark_running(
        &mut self,
        id: &str,
        pid: i32,
        pgid: i32,
        ports: Vec<u16>,
    ) -> Result<(), RegistryError> {
        let service = self.service_mut(id)?;
        service.state = ServiceState::Running { pid, pgid, ports };
        Ok(())
    }

    pub fn mark_failed(
        &mut self,
        id: &str,
        pid: i32,
        pgid: i32,
        reason: impl Into<String>,
    ) -> Result<(), RegistryError> {
        let service = self.service_mut(id)?;
        service.state = ServiceState::Failed {
            pid,
            pgid,
            reason: reason.into(),
        };
        Ok(())
    }

    pub fn mark_stopped(&mut self, id: &str) -> Result<(), RegistryError> {
        let service = self.service_mut(id)?;
        service.state = ServiceState::Stopped;
        Ok(())
    }

    pub fn update_ports(&mut self, id: &str, ports: Vec<u16>) -> Result<(), RegistryError> {
        let service = self.service_mut(id)?;

        if let ServiceState::Running { ports: current, .. } = &mut service.state {
            *current = ports;
        }

        Ok(())
    }

    pub fn require_started(&self, id: &str) -> Result<(), RegistryError> {
        match self.status(id)? {
            ServiceState::Starting { .. }
            | ServiceState::Running { .. }
            | ServiceState::Failed { .. } => Ok(()),
            ServiceState::Stopped => Err(RegistryError::AlreadyStopped(id.to_string())),
        }
    }

    fn ensure_can_insert(&self, config: &ServiceConfig) -> Result<(), RegistryError> {
        if self.services.contains_key(&config.id) {
            return Err(RegistryError::AlreadyRegistered(config.id.clone()));
        }

        self.ensure_host_available(&config.id, &config.host)
    }

    fn ensure_exists(&self, id: &str) -> Result<(), RegistryError> {
        if self.services.contains_key(id) {
            Ok(())
        } else {
            Err(RegistryError::UnknownService(id.to_string()))
        }
    }

    fn ensure_host_available(&self, id: &str, host: &str) -> Result<(), RegistryError> {
        if self
            .services
            .values()
            .any(|service| service.config.id != id && service.config.host == host)
        {
            return Err(RegistryError::HostAlreadyRegistered(host.to_string()));
        }

        Ok(())
    }

    fn service(&self, id: &str) -> Result<&Service, RegistryError> {
        self.resolve(id)
            .ok_or_else(|| RegistryError::UnknownService(id.to_string()))
    }

    fn service_mut(&mut self, id: &str) -> Result<&mut Service, RegistryError> {
        self.resolve_mut(id)
            .ok_or_else(|| RegistryError::UnknownService(id.to_string()))
    }
}
