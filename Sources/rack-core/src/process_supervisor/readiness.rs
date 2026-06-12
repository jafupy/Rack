use super::is_running;
use crate::process::LaunchPlan;
use crate::process_readiness::{loopback_listening_ports, probe_port, probe_unix_socket};
use crate::routes::{update_route_port, update_route_socket_path};
use crate::{emit, EventCallback};
use std::collections::HashSet;
use std::thread;
use std::time::Duration;

pub(super) fn spawn_readiness_monitor(
    id: String,
    pid: u32,
    plan: LaunchPlan,
    explicit_port: Option<u16>,
    port_snapshot: HashSet<u16>,
    callback: Option<EventCallback>,
    context: usize,
) {
    thread::spawn(move || {
        for _ in 0..120 {
            thread::sleep(Duration::from_millis(500));
            if !is_running(&id) {
                return;
            }

            if let Some(route_target) = ready_route_target(&plan, explicit_port, &port_snapshot) {
                match route_target {
                    ReadyRouteTarget::Socket(path) => {
                        let _ = update_route_socket_path(&plan.subdomain, &path);
                    }
                    ReadyRouteTarget::Port(port) => {
                        let _ = update_route_port(&plan.subdomain, port);
                    }
                }
                emit_status(callback, context, "server.ready", &id, pid, &plan, None);
                return;
            }
        }

        emit_status(
            callback,
            context,
            "server.failed",
            &id,
            pid,
            &plan,
            Some("Did not start within 60s"),
        );
    });
}

enum ReadyRouteTarget {
    Socket(String),
    Port(u16),
}

fn ready_route_target(
    plan: &LaunchPlan,
    explicit_port: Option<u16>,
    port_snapshot: &HashSet<u16>,
) -> Option<ReadyRouteTarget> {
    if plan.use_bridge {
        return probe_unix_socket(&plan.socket_path)
            .then(|| ReadyRouteTarget::Socket(plan.socket_path.clone()));
    }

    if explicit_port.is_some() {
        return probe_port(plan.port).then_some(ReadyRouteTarget::Port(plan.port));
    }

    let current = loopback_listening_ports();
    current
        .difference(port_snapshot)
        .copied()
        .filter(|port| *port > 1024)
        .min()
        .map(ReadyRouteTarget::Port)
}

fn emit_status(
    callback: Option<EventCallback>,
    context: usize,
    event_type: &str,
    id: &str,
    pid: u32,
    plan: &LaunchPlan,
    message: Option<&str>,
) {
    if let Some(callback) = callback {
        emit(
            callback,
            context,
            &serde_json::json!({
                "type": event_type,
                "payload": {
                    "id": id,
                    "pid": pid,
                    "plan": plan,
                    "message": message,
                }
            })
            .to_string(),
        );
    }
}
