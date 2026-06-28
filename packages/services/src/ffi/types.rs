use std::os::raw::c_char;

use crate::snapshot::{ServiceSnapshot, Snapshot, StateSnapshot};

use super::status::{free_string, string_ptr, ABI_VERSION};

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RackServicesStateKind {
    Stopped = 0,
    Starting = 1,
    Running = 2,
    Failed = 3,
}

#[repr(C)]
pub struct RackServicesSnapshot {
    pub abi_version: u32,
    pub has_proxy_port: u8,
    pub proxy_port: u16,
    pub services_len: usize,
    pub services: *mut RackServicesServiceSnapshot,
}

#[repr(C)]
pub struct RackServicesServiceSnapshot {
    pub abi_version: u32,
    pub state: RackServicesStateKind,
    pub auto_start: u8,
    pub pid: i32,
    pub pgid: i32,
    pub id: *mut c_char,
    pub name: *mut c_char,
    pub host: *mut c_char,
    pub run: *mut c_char,
    pub working_dir: *mut c_char,
    pub ports_len: usize,
    pub ports: *mut u16,
}

impl RackServicesSnapshot {
    pub fn from_native(snapshot: Snapshot) -> Self {
        let (has_proxy_port, proxy_port) = match snapshot.proxy_port {
            Some(port) => (1, port),
            None => (0, 0),
        };
        let services = snapshot
            .services
            .into_iter()
            .map(RackServicesServiceSnapshot::from_native)
            .collect::<Vec<_>>();
        let services_len = services.len();
        let services = boxed_slice_into_raw(services);

        Self {
            abi_version: ABI_VERSION,
            has_proxy_port,
            proxy_port,
            services_len,
            services,
        }
    }
}

impl RackServicesServiceSnapshot {
    fn from_native(service: ServiceSnapshot) -> Self {
        let (state, pid, pgid, ports) = state_parts(service.state);
        let ports_len = ports.len();
        let ports = boxed_slice_into_raw(ports);

        Self {
            abi_version: ABI_VERSION,
            state,
            auto_start: u8::from(service.auto_start),
            pid,
            pgid,
            id: string_ptr(service.id),
            name: string_ptr(service.name),
            host: string_ptr(service.host),
            run: string_ptr(service.run),
            working_dir: string_ptr(service.working_dir),
            ports_len,
            ports,
        }
    }
}

pub unsafe fn free_snapshot(snapshot: *mut RackServicesSnapshot) {
    if snapshot.is_null() {
        return;
    }

    let snapshot = Box::from_raw(snapshot);
    if !snapshot.services.is_null() {
        let services = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            snapshot.services,
            snapshot.services_len,
        ));
        for service in Vec::from(services) {
            free_service(service);
        }
    }
}

fn free_service(service: RackServicesServiceSnapshot) {
    unsafe {
        free_string(service.id);
        free_string(service.name);
        free_string(service.host);
        free_string(service.run);
        free_string(service.working_dir);

        if !service.ports.is_null() {
            let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                service.ports,
                service.ports_len,
            ));
        }
    }
}

fn state_parts(state: StateSnapshot) -> (RackServicesStateKind, i32, i32, Vec<u16>) {
    match state {
        StateSnapshot::Stopped => (RackServicesStateKind::Stopped, 0, 0, Vec::new()),
        StateSnapshot::Starting { pid, pgid } => {
            (RackServicesStateKind::Starting, pid, pgid, Vec::new())
        }
        StateSnapshot::Running { pid, pgid, ports } => {
            (RackServicesStateKind::Running, pid, pgid, ports)
        }
        StateSnapshot::Failed { pid, pgid, .. } => {
            (RackServicesStateKind::Failed, pid, pgid, Vec::new())
        }
    }
}

fn boxed_slice_into_raw<T>(values: Vec<T>) -> *mut T {
    let mut values = values.into_boxed_slice();
    let ptr = values.as_mut_ptr();
    std::mem::forget(values);
    ptr
}
