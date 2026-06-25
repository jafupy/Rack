use std::{
    collections::HashMap,
    sync::mpsc::{Receiver, RecvTimeoutError},
    time::Duration,
};

use crate::{
    process::{Process, ProcessError},
    registry::Registry,
};

use super::{
    log::{append_service_log, clear_service_log},
    Message, SupervisorError,
};

const POLL_INTERVAL: Duration = Duration::from_millis(250);
const MAX_LOG_LINES: usize = 400;

pub(super) fn run(mut registry: Registry, commands: Receiver<Message>) {
    let mut processes = HashMap::new();
    let mut logs = HashMap::new();

    loop {
        collect_output(&mut processes, &mut logs);
        reap_exited(&mut registry, &mut processes, &mut logs);

        match commands.recv_timeout(POLL_INTERVAL) {
            Ok(Message::Shutdown) | Err(RecvTimeoutError::Disconnected) => {
                stop_all(processes);
                break;
            }
            Ok(command) => handle(command, &mut registry, &mut processes, &mut logs),
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}

fn handle(
    command: Message,
    registry: &mut Registry,
    processes: &mut HashMap<String, Process>,
    logs: &mut HashMap<String, String>,
) {
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
        Message::Log { id, reply } => {
            if let Some(process) = processes.get_mut(&id) {
                append_output(logs, &id, process.drain_output());
            }
            let _ = reply.send(Ok(logs.get(&id).cloned().unwrap_or_default()));
        }
        Message::Start { id, reply } => {
            logs.insert(id.clone(), String::new());
            clear_service_log(&id);
            let _ = reply.send(start_service(registry, processes, &id));
        }
        Message::Stop { id, reply } => {
            let _ = reply.send(stop_service(registry, processes, &id));
        }
        Message::Restart { id, reply } => {
            logs.insert(id.clone(), String::new());
            clear_service_log(&id);
            let _ = reply.send(restart_service(registry, processes, &id));
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

fn restart_service(
    registry: &mut Registry,
    processes: &mut HashMap<String, Process>,
    id: &str,
) -> Result<(), SupervisorError> {
    if !matches!(registry.status(id)?, crate::registry::ServiceState::Stopped) {
        stop_service(registry, processes, id)?;
    }

    start_service(registry, processes, id)
}

fn stop_service(
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

fn collect_output(processes: &mut HashMap<String, Process>, logs: &mut HashMap<String, String>) {
    for (id, process) in processes {
        append_output(logs, id, process.drain_output());
    }
}

fn reap_exited(
    registry: &mut Registry,
    processes: &mut HashMap<String, Process>,
    logs: &mut HashMap<String, String>,
) {
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
            append_output(logs, id, process.drain_output());
            let _ = registry.mark_stopped(id);
            false
        }
        Err(error) => {
            eprintln!("failed to poll service process for {id}: {error}");
            append_output(logs, id, process.drain_output());
            let _ = registry.mark_stopped(id);
            false
        }
    });
}

fn append_output(logs: &mut HashMap<String, String>, id: &str, output: Vec<String>) {
    if output.is_empty() {
        return;
    }

    append_service_log(id, &output);

    let log = logs.entry(id.to_string()).or_default();
    for chunk in output {
        log.push_str(&chunk);
    }

    let lines = log.lines().count();
    if lines > MAX_LOG_LINES {
        *log = log
            .lines()
            .skip(lines - MAX_LOG_LINES)
            .collect::<Vec<_>>()
            .join("\n");
    }
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
    for (id, mut process) in processes {
        let _ = process.kill(&id);
    }
}
