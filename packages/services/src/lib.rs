pub mod process;
pub mod registry;
pub mod supervisor;

use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    os::raw::c_char,
    sync::Mutex,
};

use rack_core::config::{self, Service as ServiceConfig};
use serde::Serialize;

use registry::{Registry, ServiceState, ServiceView};
use supervisor::Supervisor;

static RUNTIME: Mutex<Option<RackRuntime>> = Mutex::new(None);

struct RackRuntime {
    supervisor: Supervisor,
    configs: HashMap<String, ServiceConfig>,
}

#[derive(Serialize)]
struct Snapshot {
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

        for service in config.services {
            registry
                .register(service.clone())
                .map_err(|error| error.to_string())?;
            configs.insert(service.id.clone(), service);
        }

        let supervisor = Supervisor::start(registry);
        let mut runtime = RUNTIME.lock().map_err(|error| error.to_string())?;
        *runtime = Some(RackRuntime {
            supervisor,
            configs,
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
        let services = runtime
            .supervisor
            .list()
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|view| snapshot_service(view, &runtime.configs))
            .collect::<Vec<_>>();

        serde_json::to_string(&Snapshot { services }).map_err(|error| error.to_string())
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
    ffi_result(|| {
        let id = unsafe { c_string(id) }?;
        let runtime = RUNTIME.lock().map_err(|error| error.to_string())?;
        let runtime = runtime
            .as_ref()
            .ok_or_else(|| "rack services runtime has not been initialized".to_string())?;
        action(runtime, &id)?;
        Ok(String::new())
    })
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
