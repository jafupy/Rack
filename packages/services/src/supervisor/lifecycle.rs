use std::{collections::HashMap, time::Duration};

use crate::{
    process::{Process, ProcessError},
    registry::{Registry, ServiceState},
};

use super::SupervisorError;

const READINESS_TIMEOUT: Duration = Duration::from_secs(30);
const READINESS_TIMEOUT_REASON: &str =
    "readiness timeout: service did not open a listening port within 30 seconds";

pub(super) fn start_service(
    registry: &mut Registry,
    processes: &mut HashMap<String, Process>,
    id: &str,
) -> Result<(), SupervisorError> {
    registry.mark_starting(id)?;
    let config = registry.config(id)?;

    if processes.contains_key(id) {
        return Err(ProcessError::UnexpectedHandle(id.to_string()).into());
    }

    let mut process = Process::spawn(id, &config).inspect_err(|_| {
        let _ = registry.mark_stopped(id);
    })?;

    if let Err(error) = registry.mark_spawned(id, process.pid(), process.pgid()) {
        let _ = process.kill(id);
        return Err(error.into());
    }

    processes.insert(id.to_string(), process);
    Ok(())
}

pub(super) fn restart_service(
    registry: &mut Registry,
    processes: &mut HashMap<String, Process>,
    id: &str,
) -> Result<(), SupervisorError> {
    if !matches!(registry.status(id)?, ServiceState::Stopped) {
        stop_service(registry, processes, id)?;
    }

    start_service(registry, processes, id)
}

pub(super) fn stop_service(
    registry: &mut Registry,
    processes: &mut HashMap<String, Process>,
    id: &str,
) -> Result<(), SupervisorError> {
    registry.require_started(id)?;

    let Some(process) = processes.get_mut(id) else {
        return Err(ProcessError::MissingHandle(id.to_string()).into());
    };

    process.kill(id)?;
    processes.remove(id);
    registry.mark_stopped(id)?;
    Ok(())
}

pub(super) fn unregister_service(
    registry: &mut Registry,
    processes: &mut HashMap<String, Process>,
    logs: &mut HashMap<String, String>,
    id: &str,
) -> Result<rack_core::config::Service, SupervisorError> {
    if !matches!(registry.status(id)?, ServiceState::Stopped) {
        stop_service(registry, processes, id)?;
    }

    logs.remove(id);
    registry.unregister(id).map_err(Into::into)
}

pub(super) fn update_running_state(
    registry: &mut Registry,
    id: &str,
    process: &Process,
) -> Result<(), SupervisorError> {
    match registry.status(id)? {
        ServiceState::Starting { .. } => {
            let ports = process.ports(id)?;
            if !ports.is_empty() {
                registry.mark_running(id, process.pid(), process.pgid(), ports)?;
            } else if process.readiness_timed_out(READINESS_TIMEOUT) {
                registry.mark_failed(
                    id,
                    process.pid(),
                    process.pgid(),
                    READINESS_TIMEOUT_REASON,
                )?;
            }
        }
        ServiceState::Running { .. } => {
            let ports = process.ports(id)?;
            if !ports.is_empty() {
                registry.update_ports(id, ports)?;
            }
        }
        ServiceState::Failed { .. } => {}
        ServiceState::Stopped => {
            return Err(ProcessError::RegistryDesync(id.to_string()).into());
        }
    }

    Ok(())
}

pub(super) fn stop_all(processes: HashMap<String, Process>) {
    for (id, mut process) in processes {
        let _ = process.kill(&id);
    }
}
