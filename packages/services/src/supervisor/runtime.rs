use std::{
    collections::HashMap,
    sync::mpsc::{Receiver, RecvTimeoutError},
    time::Duration,
};

use crate::{
    process::{Process, ProcessError},
    registry::Registry,
};

use super::{Message, SupervisorError};

const POLL_INTERVAL: Duration = Duration::from_millis(250);

pub(super) fn run(mut registry: Registry, commands: Receiver<Message>) {
    let mut processes = HashMap::new();

    loop {
        reap_exited(&mut registry, &mut processes);

        match commands.recv_timeout(POLL_INTERVAL) {
            Ok(Message::Shutdown) | Err(RecvTimeoutError::Disconnected) => {
                stop_all(processes);
                break;
            }
            Ok(command) => handle(command, &mut registry, &mut processes),
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}

fn handle(command: Message, registry: &mut Registry, processes: &mut HashMap<String, Process>) {
    match command {
        Message::Register { config, reply } => {
            let _ = reply.send(registry.register(config).map_err(Into::into));
        }
        Message::List { reply } => {
            let _ = reply.send(Ok(registry.list()));
        }
        Message::Status { id, reply } => {
            let _ = reply.send(registry.status(&id).map_err(Into::into));
        }
        Message::Start { id, reply } => {
            let _ = reply.send(start_service(registry, processes, &id));
        }
        Message::Stop { id, reply } => {
            let _ = reply.send(stop_service(registry, processes, &id));
        }
        Message::Shutdown => {}
    }
}

fn start_service(
    registry: &mut Registry,
    processes: &mut HashMap<String, Process>,
    id: &str,
) -> Result<(), SupervisorError> {
    registry.mark_starting(id)?;
    let config = registry.config(id)?;

    if processes.contains_key(id) {
        return Err(ProcessError::UnexpectedHandle(id.to_string()).into());
    }

    let process = Process::spawn(id, &config).inspect_err(|_| {
        let _ = registry.mark_stopped(id);
    })?;

    if let Err(error) = registry.mark_spawned(id, process.pid(), process.pgid()) {
        let _ = process.kill(id);
        return Err(error.into());
    }

    processes.insert(id.to_string(), process);
    Ok(())
}

fn stop_service(
    registry: &mut Registry,
    processes: &mut HashMap<String, Process>,
    id: &str,
) -> Result<(), SupervisorError> {
    registry.require_started(id)?;

    let Some(process) = processes.remove(id) else {
        return Err(ProcessError::MissingHandle(id.to_string()).into());
    };

    process.kill(id)?;
    registry.mark_stopped(id)?;
    Ok(())
}

fn reap_exited(registry: &mut Registry, processes: &mut HashMap<String, Process>) {
    processes.retain(|id, process| match process.has_exited() {
        Ok(false) => {
            match process.ports(id) {
                Ok(ports) => {
                    if let Err(error) = update_ports(registry, id, process, ports) {
                        eprintln!("failed to update service ports for {id}: {error}");
                    }
                }
                Err(error) => eprintln!("failed to inspect service ports for {id}: {error}"),
            }
            true
        }
        Ok(true) => {
            let _ = registry.mark_stopped(id);
            false
        }
        Err(error) => {
            eprintln!("failed to poll service process for {id}: {error}");
            let _ = registry.mark_stopped(id);
            false
        }
    });
}

fn update_ports(
    registry: &mut Registry,
    id: &str,
    process: &Process,
    ports: Vec<u16>,
) -> Result<(), SupervisorError> {
    if ports.is_empty() {
        return Ok(());
    }

    match registry.status(id)? {
        crate::registry::ServiceState::Starting { .. } => {
            registry.mark_running(id, process.pid(), process.pgid(), ports)?;
        }
        crate::registry::ServiceState::Running { .. } => {
            registry.update_ports(id, ports)?;
        }
        crate::registry::ServiceState::Stopped => {
            return Err(ProcessError::RegistryDesync(id.to_string()).into());
        }
    }

    Ok(())
}

fn stop_all(processes: HashMap<String, Process>) {
    for (id, process) in processes {
        let _ = process.kill(&id);
    }
}
