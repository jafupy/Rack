mod configuration;
mod hooks;
mod proxy;
mod services;

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use rack_core::config::{self, Service as ServiceConfig};

use self::proxy::{bind_proxy, target_table};
use crate::{
    control::ControlServer,
    hooks::{self as hook_registry, HookScheduler, HookSummary},
    registry::{Registry, ServiceView},
    snapshot::{snapshot_service, Snapshot},
    supervisor::Supervisor,
};

pub(crate) type SharedServiceConfigs = Arc<RwLock<HashMap<String, ServiceConfig>>>;

pub struct RackRuntime {
    supervisor: Supervisor,
    configs: SharedServiceConfigs,
    proxy_runtime: tokio::runtime::Runtime,
    proxy: Option<rack_proxy::ProxyServer>,
    control: Option<ControlServer>,
    hooks: Vec<HookSummary>,
    hook_scheduler: Option<HookScheduler>,
}

impl RackRuntime {
    pub fn init() -> Result<Self, String> {
        let config = config::load().map_err(|error| error.to_string())?;
        let mut registry = Registry::new();
        let mut configs = HashMap::new();
        let auto_start = auto_start_ids(&config.services);

        for service in config.services {
            registry
                .register(service.clone())
                .map_err(|error| error.to_string())?;
            configs.insert(service.id.clone(), service);
        }

        let supervisor = Supervisor::start(registry);
        for id in auto_start {
            supervisor
                .start_service(id)
                .map_err(|error| error.to_string())?;
        }

        let proxy_runtime = tokio::runtime::Runtime::new().map_err(|error| error.to_string())?;
        let proxy = bind_proxy(&proxy_runtime)?;
        let deployed_hooks = hook_registry::load_deployed(&proxy.hooks());
        let hook_scheduler = Some(HookScheduler::start(deployed_hooks.crons));
        let configs = Arc::new(RwLock::new(configs));
        let control = ControlServer::start(
            supervisor.clone(),
            configs.clone(),
            proxy.addr().port(),
            proxy.targets(),
        )?;

        Ok(Self {
            supervisor,
            configs,
            proxy_runtime,
            proxy: Some(proxy),
            control: Some(control),
            hooks: deployed_hooks.summaries,
            hook_scheduler,
        })
    }

    pub fn snapshot_json(&self) -> Result<String, String> {
        serde_json::to_string(&self.snapshot()?).map_err(|error| error.to_string())
    }

    pub fn snapshot(&self) -> Result<Snapshot, String> {
        let views = self.supervisor.list().map_err(|error| error.to_string())?;
        self.refresh_proxy_targets(&views);
        let configs = self.configs.read().map_err(|error| error.to_string())?;
        Ok(Snapshot {
            proxy_port: self.proxy.as_ref().map(|proxy| proxy.addr().port()),
            services: views
                .into_iter()
                .map(|view| snapshot_service(view, &configs))
                .collect(),
        })
    }

    fn refresh_after_command(&self) -> Result<(), String> {
        let views = self.supervisor.list().map_err(|error| error.to_string())?;
        self.refresh_proxy_targets(&views);
        Ok(())
    }

    fn refresh_proxy_targets(&self, services: &[ServiceView]) {
        if let Some(proxy) = &self.proxy {
            proxy.targets().update(target_table(services));
        }
    }
}

impl Drop for RackRuntime {
    fn drop(&mut self) {
        self.hook_scheduler.take();
        let _ = self.supervisor.shutdown();
        self.control.take();
        if let Some(proxy) = self.proxy.take() {
            let _ = self.proxy_runtime.block_on(proxy.shutdown());
        }
    }
}

pub fn auto_start_ids(services: &[ServiceConfig]) -> Vec<String> {
    services
        .iter()
        .filter(|service| service.auto_start)
        .map(|service| service.id.clone())
        .collect()
}
