use anyhow::Result;
use rack_core::config;
use rack_services::snapshot::{ServiceSnapshot, Snapshot, StateSnapshot};

pub fn print_snapshot(snapshot: Snapshot) -> Result<()> {
    if snapshot.services.is_empty() {
        println!("No services configured");
        return Ok(());
    }

    for service in snapshot.services {
        print_service(&service, snapshot.proxy_port);
    }
    Ok(())
}

pub fn print_config_services() -> Result<()> {
    let config = config::load()?;
    if config.services.is_empty() {
        println!("No services configured");
        return Ok(());
    }

    for service in config.services {
        print_service(
            &ServiceSnapshot {
                id: service.id,
                name: service.name,
                host: service.host,
                run: service.run,
                working_dir: service.working_dir,
                auto_start: service.auto_start,
                state: StateSnapshot::Stopped,
            },
            None,
        );
    }
    Ok(())
}

fn print_service(service: &ServiceSnapshot, proxy_port: Option<u16>) {
    let auto_start = if service.auto_start {
        " auto-start"
    } else {
        ""
    };
    println!(
        "{}\t{}\t{}\t{}\t{}{}{}",
        service.id,
        service.name,
        state_label(&service.state),
        service_url(service, proxy_port),
        service.run,
        ports_label(&service.state),
        auto_start
    );
}

fn service_url(service: &ServiceSnapshot, proxy_port: Option<u16>) -> String {
    match proxy_port {
        Some(80) | None => format!("http://{}.localhost", service.host),
        Some(port) => format!("http://{}.localhost:{port}", service.host),
    }
}

fn ports_label(state: &StateSnapshot) -> String {
    match state {
        StateSnapshot::Running { ports, .. } if !ports.is_empty() => format!(
            " ports={}",
            ports
                .iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(",")
        ),
        _ => String::new(),
    }
}

fn state_label(state: &StateSnapshot) -> &'static str {
    match state {
        StateSnapshot::Stopped => "stopped",
        StateSnapshot::Starting { .. } => "starting",
        StateSnapshot::Running { .. } => "running",
        StateSnapshot::Failed { .. } => "failed",
    }
}
