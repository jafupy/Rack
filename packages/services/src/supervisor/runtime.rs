use std::{
    collections::HashMap,
    sync::mpsc::{Receiver, RecvTimeoutError},
    time::Duration,
};

use crate::{process::Process, registry::Registry};

use super::{
    lifecycle::{
        restart_service, start_service, stop_all, stop_service, unregister_service,
        update_running_state,
    },
    log::clear_service_log,
    output::{append_output, collect_output},
    Message,
};

const POLL_INTERVAL: Duration = Duration::from_millis(250);

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
        Message::Update { config, reply } => {
            let _ = reply.send(registry.update(config).map_err(Into::into));
        }
        Message::Unregister { id, reply } => {
            let _ = reply.send(unregister_service(registry, processes, logs, &id));
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

fn reap_exited(
    registry: &mut Registry,
    processes: &mut HashMap<String, Process>,
    logs: &mut HashMap<String, String>,
) {
    processes.retain(|id, process| match process.has_exited() {
        Ok(false) => {
            if let Err(error) = update_running_state(registry, id, process) {
                eprintln!("failed to update service state for {id}: {error}");
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
