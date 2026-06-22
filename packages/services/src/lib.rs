pub mod process;
pub mod registry;
pub mod supervisor;

use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    os::raw::c_char,
    sync::Mutex,
};

use rack_core::config::{self, Service as ServiceConfig};
use rack_proxy::{ProxyServer, ServiceTarget, TargetTable};
use serde::Serialize;

use registry::{Registry, ServiceState, ServiceView};
use supervisor::Supervisor;

static RUNTIME: Mutex<Option<RackRuntime>> = Mutex::new(None);

struct RackRuntime {
    supervisor: Supervisor,
    configs: HashMap<String, ServiceConfig>,
    proxy_runtime: tokio::runtime::Runtime,
    proxy: Option<ProxyServer>,
}

impl Drop for RackRuntime {
    fn drop(&mut self) {
        if let Some(proxy) = self.proxy.take() {
            let _ = self.proxy_runtime.block_on(proxy.shutdown());
        }
    }
}

#[derive(Serialize)]
struct Snapshot {
    proxy_port: Option<u16>,
    services: Vec<ServiceSnapshot>,
}

#[derive(Serialize)]
struct ServiceSnapshot {
    id: String,
    name: String,
    host: String,
    run: String,
    working_dir: String,
    auto_start: bool,
    state: StateSnapshot,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum StateSnapshot {
    Stopped,
    Starting {
        pid: i32,
        pgid: i32,
    },
    Running {
        pid: i32,
        pgid: i32,
        ports: Vec<u16>,
    },
}

#[no_mangle]
pub extern "C" fn rack_services_init() -> *mut c_char {
    ffi_result(|| {
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
        let mut runtime = RUNTIME.lock().map_err(|error| error.to_string())?;
        *runtime = Some(RackRuntime {
            supervisor,
            configs,
            proxy_runtime,
            proxy: Some(proxy),
        });
        Ok(String::new())
    })
}

#[no_mangle]
pub extern "C" fn rack_services_snapshot_json() -> *mut c_char {
    ffi_result(|| {
        let runtime = RUNTIME.lock().map_err(|error| error.to_string())?;
        let runtime = runtime
            .as_ref()
            .ok_or_else(|| "rack services runtime has not been initialized".to_string())?;
        let views = runtime
            .supervisor
            .list()
            .map_err(|error| error.to_string())?;
        refresh_proxy_targets(runtime, &views);
        let services = views
            .into_iter()
            .map(|view| snapshot_service(view, &runtime.configs))
            .collect::<Vec<_>>();

        serde_json::to_string(&Snapshot {
            proxy_port: runtime.proxy.as_ref().map(|proxy| proxy.addr().port()),
            services,
        })
        .map_err(|error| error.to_string())
    })
}

#[no_mangle]
pub extern "C" fn rack_services_start_service(id: *const c_char) -> *mut c_char {
    with_service_id(id, |runtime, id| {
        runtime
            .supervisor
            .start_service(id)
            .map_err(|error| error.to_string())
    })
}

#[no_mangle]
pub extern "C" fn rack_services_stop_service(id: *const c_char) -> *mut c_char {
    with_service_id(id, |runtime, id| {
        runtime
            .supervisor
            .stop_service(id)
            .map_err(|error| error.to_string())
    })
}

#[no_mangle]
pub extern "C" fn rack_services_log(id: *const c_char) -> *mut c_char {
    with_service_id_value(id, |runtime, id| {
        runtime
            .supervisor
            .log(id)
            .map_err(|error| error.to_string())
    })
}

#[no_mangle]
pub extern "C" fn rack_services_shutdown() -> *mut c_char {
    ffi_result(|| {
        let mut runtime = RUNTIME.lock().map_err(|error| error.to_string())?;
        *runtime = None;
        Ok(String::new())
    })
}

#[no_mangle]
pub unsafe extern "C" fn rack_services_string_free(value: *mut c_char) {
    if !value.is_null() {
        let _ = CString::from_raw(value);
    }
}

fn with_service_id(
    id: *const c_char,
    action: impl FnOnce(&RackRuntime, &str) -> Result<(), String>,
) -> *mut c_char {
    with_service_id_value(id, |runtime, id| {
        action(runtime, id)?;
        Ok(String::new())
    })
}

fn with_service_id_value(
    id: *const c_char,
    action: impl FnOnce(&RackRuntime, &str) -> Result<String, String>,
) -> *mut c_char {
    ffi_result(|| {
        let id = unsafe { c_string(id) }?;
        let runtime = RUNTIME.lock().map_err(|error| error.to_string())?;
        let runtime = runtime
            .as_ref()
            .ok_or_else(|| "rack services runtime has not been initialized".to_string())?;
        let output = action(runtime, &id)?;
        let views = runtime
            .supervisor
            .list()
            .map_err(|error| error.to_string())?;
        refresh_proxy_targets(runtime, &views);
        Ok(output)
    })
}

fn auto_start_ids(services: &[ServiceConfig]) -> Vec<String> {
    services
        .iter()
        .filter(|service| service.auto_start)
        .map(|service| service.id.clone())
        .collect()
}

fn snapshot_service(
    view: ServiceView,
    configs: &HashMap<String, ServiceConfig>,
) -> ServiceSnapshot {
    let config = configs.get(&view.id);
    ServiceSnapshot {
        id: view.id,
        name: view.name,
        host: view.host,
        run: config.map(|config| config.run.clone()).unwrap_or_default(),
        working_dir: config
            .map(|config| config.working_dir.clone())
            .unwrap_or_else(|| "~".to_string()),
        auto_start: config.map(|config| config.auto_start).unwrap_or_default(),
        state: snapshot_state(view.state),
    }
}

fn snapshot_state(state: ServiceState) -> StateSnapshot {
    match state {
        ServiceState::Stopped => StateSnapshot::Stopped,
        ServiceState::Starting { pid, pgid } => StateSnapshot::Starting { pid, pgid },
        ServiceState::Running { pid, pgid, ports } => StateSnapshot::Running { pid, pgid, ports },
    }
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

fn refresh_proxy_targets(runtime: &RackRuntime, services: &[ServiceView]) {
    let targets = services.iter().filter_map(service_target);
    if let Some(proxy) = &runtime.proxy {
        proxy.targets().update(TargetTable::new(targets));
    }
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

fn ffi_result(action: impl FnOnce() -> Result<String, String>) -> *mut c_char {
    let output = match action() {
        Ok(value) => value,
        Err(error) => format!("ERROR:{error}"),
    };

    CString::new(output)
        .unwrap_or_else(|_| CString::new("ERROR:response contained nul byte").unwrap())
        .into_raw()
}

unsafe fn c_string(value: *const c_char) -> Result<String, String> {
    if value.is_null() {
        return Err("expected non-null service id".to_string());
    }

    CStr::from_ptr(value)
        .to_str()
        .map(str::to_string)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_auto_start_service_ids_in_config_order() {
        let services = vec![
            service("web", false),
            service("api", true),
            service("worker", true),
        ];

        assert_eq!(auto_start_ids(&services), vec!["api", "worker"]);
    }

    fn service(id: &str, auto_start: bool) -> ServiceConfig {
        ServiceConfig {
            id: id.to_string(),
            name: id.to_string(),
            host: id.to_string(),
            run: "sleep 1".to_string(),
            working_dir: "~".to_string(),
            auto_start,
        }
    }
}
