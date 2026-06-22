use anyhow::{bail, Result};
use rack_core::config;
use rack_services::{
    control::{Client, Command, Request, Response},
    snapshot::{ServiceSnapshot, Snapshot, StateSnapshot},
};

fn main() {
    if let Err(error) = run(std::env::args().skip(1).collect()) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<()> {
    match args.as_slice() {
        [] => {
            print_help();
            Ok(())
        }
        [arg] if arg == "help" || arg == "--help" || arg == "-h" => {
            print_help();
            Ok(())
        }
        [scope] if service_scope(scope) => list_services(),
        [scope, command] if service_scope(scope) && command == "list" => list_services(),
        [scope, command, id] if service_scope(scope) && command == "start" => {
            service_command(Command::Start, id)
        }
        [scope, command, id] if service_scope(scope) && command == "stop" => {
            service_command(Command::Stop, id)
        }
        [scope, command, id] if service_scope(scope) && command == "restart" => {
            service_command(Command::Restart, id)
        }
        [scope, command, id] if service_scope(scope) && command == "log" => service_log(id),
        [scope, command, ..] if service_scope(scope) => {
            bail!("unsupported service command `{command}`; try `rack help`")
        }
        [command, ..] => bail!("unsupported command `{command}`; try `rack help`"),
    }
}

fn service_scope(value: &str) -> bool {
    value == "service" || value == "services"
}

fn list_services() -> Result<()> {
    match control_request(Command::List, None) {
        Ok(response) => print_snapshot(response_snapshot(response)?),
        Err(_) => print_config_services(),
    }
}

fn service_command(command: Command, id: &str) -> Result<()> {
    let response = control_request(command, Some(id.to_string()))?;
    print_snapshot(response_snapshot(response)?)
}

fn service_log(id: &str) -> Result<()> {
    let response = control_request(Command::Log, Some(id.to_string()))?;
    if !response.ok {
        bail!(response
            .error
            .unwrap_or_else(|| "service log failed".to_string()));
    }

    print!("{}", response.log.unwrap_or_default());
    Ok(())
}

fn control_request(command: Command, id: Option<String>) -> Result<Response> {
    Client::connect_default()
        .request(Request { command, id })
        .map_err(|error| anyhow::anyhow!(error))
}

fn response_snapshot(response: Response) -> Result<Snapshot> {
    if !response.ok {
        bail!(response
            .error
            .unwrap_or_else(|| "service command failed".to_string()));
    }

    response
        .snapshot
        .ok_or_else(|| anyhow::anyhow!("service command returned no snapshot"))
}

fn print_snapshot(snapshot: Snapshot) -> Result<()> {
    if snapshot.services.is_empty() {
        println!("No services configured");
        return Ok(());
    }

    for service in snapshot.services {
        print_service(&service);
    }

    Ok(())
}

fn print_config_services() -> Result<()> {
    let config = config::load()?;

    if config.services.is_empty() {
        println!("No services configured");
        return Ok(());
    }

    for service in config.services {
        let service = ServiceSnapshot {
            id: service.id,
            name: service.name,
            host: service.host,
            run: service.run,
            working_dir: service.working_dir,
            auto_start: service.auto_start,
            state: StateSnapshot::Stopped,
        };
        print_service(&service);
    }

    Ok(())
}

fn print_service(service: &ServiceSnapshot) {
    let auto_start = if service.auto_start {
        " auto-start"
    } else {
        ""
    };
    println!(
        "{}\t{}\t{}\thttp://{}.localhost\t{}{}",
        service.id,
        service.name,
        state_label(&service.state),
        service.host,
        service.run,
        auto_start
    );
}

fn state_label(state: &StateSnapshot) -> &'static str {
    match state {
        StateSnapshot::Stopped => "stopped",
        StateSnapshot::Starting { .. } => "starting",
        StateSnapshot::Running { .. } => "running",
    }
}

fn print_help() {
    println!(
        "Rack\n\nUsage:\n  rack service list\n  rack service start <id>\n  rack service stop <id>\n  rack service restart <id>\n  rack service log <id>\n  rack services\n\nCommands:\n  service list          List services\n  service start <id>    Start a running Rack service\n  service stop <id>     Stop a running Rack service\n  service restart <id>  Restart a running Rack service\n  service log <id>      Print captured service logs"
    );
}
