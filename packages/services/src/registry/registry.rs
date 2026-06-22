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
        if self.services.contains_key(&config.id) {
            return Err(RegistryError::AlreadyRegistered(config.id));
        }

        if self
            .services
            .values()
            .any(|service| service.config.host == config.host)
        {
            return Err(RegistryError::HostAlreadyRegistered(config.host));
        }

        self.services
            .insert(config.id.clone(), Service::new(config));
        Ok(())
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
            ServiceState::Starting { .. } | ServiceState::Running { .. } => Ok(()),
            ServiceState::Stopped => Err(RegistryError::AlreadyStopped(id.to_string())),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_services_by_id() {
        let mut registry = Registry::new();
        registry.register(service("api", "api")).unwrap();

        assert_eq!(registry.resolve("api").unwrap().config.name, "API");
    }

    #[test]
    fn rejects_duplicate_ids() {
        let mut registry = Registry::new();
        registry.register(service("api", "api")).unwrap();

        let error = registry.register(service("api", "worker")).unwrap_err();

        assert!(matches!(error, RegistryError::AlreadyRegistered(id) if id == "api"));
    }

    #[test]
    fn rejects_duplicate_hosts() {
        let mut registry = Registry::new();
        registry.register(service("api", "api")).unwrap();

        let error = registry.register(service("worker", "api")).unwrap_err();

        assert!(matches!(error, RegistryError::HostAlreadyRegistered(host) if host == "api"));
    }

    #[test]
    fn transitions_from_stopped_to_starting_to_running() {
        let mut registry = registered();

        registry.mark_starting("api").unwrap();
        registry.mark_spawned("api", 10, 10).unwrap();
        assert_eq!(
            registry.status("api").unwrap(),
            ServiceState::Starting { pid: 10, pgid: 10 }
        );

        registry.mark_running("api", 10, 10, vec![3000]).unwrap();
        assert_eq!(
            registry.status("api").unwrap(),
            ServiceState::Running {
                pid: 10,
                pgid: 10,
                ports: vec![3000]
            }
        );
    }

    #[test]
    fn require_started_accepts_starting_and_running() {
        let mut registry = registered();

        assert!(matches!(
            registry.require_started("api"),
            Err(RegistryError::AlreadyStopped(id)) if id == "api"
        ));

        registry.mark_spawned("api", 10, 10).unwrap();
        assert!(registry.require_started("api").is_ok());

        registry.mark_running("api", 10, 10, vec![3000]).unwrap();
        assert!(registry.require_started("api").is_ok());
    }

    #[test]
    fn updates_ports_only_when_running() {
        let mut registry = registered();

        registry.mark_spawned("api", 10, 10).unwrap();
        registry.update_ports("api", vec![3000]).unwrap();
        assert_eq!(
            registry.status("api").unwrap(),
            ServiceState::Starting { pid: 10, pgid: 10 }
        );

        registry.mark_running("api", 10, 10, vec![3000]).unwrap();
        registry.update_ports("api", vec![3001]).unwrap();
        assert_eq!(
            registry.status("api").unwrap(),
            ServiceState::Running {
                pid: 10,
                pgid: 10,
                ports: vec![3001]
            }
        );
    }

    fn registered() -> Registry {
        let mut registry = Registry::new();
        registry.register(service("api", "api")).unwrap();
        registry
    }

    fn service(id: &str, host: &str) -> ServiceConfig {
        ServiceConfig {
            id: id.to_string(),
            name: id.to_uppercase(),
            host: host.to_string(),
            run: "echo hi".to_string(),
            working_dir: "~".to_string(),
            auto_start: false,
        }
    }
}
