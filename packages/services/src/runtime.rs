use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

use rack_core::config::{self, Service as ServiceConfig};
use rack_proxy::{ProxyServer, ServiceTarget, TargetTable};

use crate::{
    control::ControlServer,
    hooks::{self, HookScheduler, HookSummary},
    registry::{Registry, ServiceState, ServiceView},
    snapshot::{snapshot_service, Snapshot},
    supervisor::{log::service_log_path, Supervisor},
};

pub struct RackRuntime {
    supervisor: Supervisor,
    configs: HashMap<String, ServiceConfig>,
    proxy_runtime: tokio::runtime::Runtime,
    proxy: Option<ProxyServer>,
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
        let proxy = bind_proxy(&proxy_runtime).map_err(|error| error.to_string())?;
        let deployed_hooks = hooks::load_deployed(&proxy.hooks());
        let hook_scheduler = Some(HookScheduler::start(deployed_hooks.crons));
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

    pub fn hooks_json(&self) -> Result<String, String> {
        serde_json::to_string(&self.hooks).map_err(|error| error.to_string())
    }

    pub fn snapshot(&self) -> Result<Snapshot, String> {
        let views = self.supervisor.list().map_err(|error| error.to_string())?;
        self.refresh_proxy_targets(&views);
        Ok(Snapshot {
            proxy_port: self.proxy.as_ref().map(|proxy| proxy.addr().port()),
            services: views
                .into_iter()
                .map(|view| snapshot_service(view, &self.configs))
                .collect(),
        })
    }

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
        if !self.configs.contains_key(id) {
            return Err(format!("unknown service: {id}"));
        }
        Ok(service_log_path(id).to_string_lossy().into_owned())
    }

    fn refresh_after_command(&self) -> Result<(), String> {
        let views = self.supervisor.list().map_err(|error| error.to_string())?;
        self.refresh_proxy_targets(&views);
        Ok(())
    }

    fn refresh_proxy_targets(&self, services: &[ServiceView]) {
        let targets = services.iter().filter_map(service_target);
        if let Some(proxy) = &self.proxy {
            proxy.targets().update(TargetTable::new(targets));
        }
    }
}

impl Drop for RackRuntime {
    fn drop(&mut self) {
        self.hook_scheduler.take();
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

fn bind_proxy(runtime: &tokio::runtime::Runtime) -> Result<ProxyServer, String> {
    for port in 1355..=1365 {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
        match runtime.block_on(ProxyServer::bind(addr, TargetTable::default())) {
            Ok(proxy) => return Ok(proxy),
            Err(error) => eprintln!("failed to bind proxy at {addr}: {error}"),
        }
    }

    Err("failed to bind proxy on ports 1355 through 1365".to_string())
}

fn service_target(service: &ServiceView) -> Option<ServiceTarget> {
    let ServiceState::Running { ports, .. } = &service.state else {
        return None;
    };

    Some(ServiceTarget {
        service_id: service.id.clone(),
        host: service.host.clone(),
        port: *ports.first()?,
    })
}
