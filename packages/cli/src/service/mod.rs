use anyhow::{bail, Result};
use clap::Subcommand;
use rack_core::config;
use rack_services::control::Command as ControlCommand;

mod client;
mod output;

use client::{control_request, response_snapshot};
use output::{print_config_services, print_snapshot};

#[derive(Subcommand)]
pub enum Command {
    List,
    Start {
        id: String,
    },
    Stop {
        id: String,
    },
    Restart {
        id: String,
    },
    Log {
        id: String,
    },
    Add {
        id: String,
        name: String,
        host: String,
        run: String,
        working_dir: String,
        #[arg(long)]
        auto_start: bool,
    },
    Edit {
        id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        run: Option<String>,
        #[arg(long)]
        working_dir: Option<String>,
        #[arg(long)]
        auto_start: Option<bool>,
    },
    Remove {
        id: String,
    },
    Delete {
        id: String,
    },
}

pub fn run(command: Option<Command>) -> Result<()> {
    match command.unwrap_or(Command::List) {
        Command::List => list(),
        Command::Start { id } => control(ControlCommand::Start, &resolve_service_id(&id)?),
        Command::Stop { id } => control(ControlCommand::Stop, &resolve_service_id(&id)?),
        Command::Restart { id } => control(ControlCommand::Restart, &resolve_service_id(&id)?),
        Command::Log { id } => log(&resolve_service_id(&id)?),
        Command::Add {
            id,
            name,
            host,
            run,
            working_dir,
            auto_start,
        } => add(config::Service {
            id,
            name,
            host,
            run,
            working_dir,
            auto_start,
        }),
        Command::Edit {
            id,
            name,
            host,
            run,
            working_dir,
            auto_start,
        } => edit(
            &resolve_service_id(&id)?,
            name,
            host,
            run,
            working_dir,
            auto_start,
        ),
        Command::Remove { id } | Command::Delete { id } => remove(&resolve_service_id(&id)?),
    }
}

fn list() -> Result<()> {
    match control_request(ControlCommand::List, None, None) {
        Ok(response) => print_snapshot(response_snapshot(response)?),
        Err(_) => print_config_services(),
    }
}

fn control(command: ControlCommand, id: &str) -> Result<()> {
    let response = control_request(command, Some(id.to_string()), None)?;
    print_snapshot(response_snapshot(response)?)
}

fn add(service: config::Service) -> Result<()> {
    match control_request(ControlCommand::Add, None, Some(service.clone())) {
        Ok(response) => print_snapshot(response_snapshot(response)?),
        Err(_) => {
            let mut config = config::load()?;
            config::add_service(&mut config, service.clone())?;
            let path = config::save(&config)?;
            println!("Added service `{}` to {}", service.id, path.display());
            Ok(())
        }
    }
}

fn edit(
    id: &str,
    name: Option<String>,
    host: Option<String>,
    run: Option<String>,
    working_dir: Option<String>,
    auto_start: Option<bool>,
) -> Result<()> {
    let mut config = config::load()?;
    let Some(current) = config.services.iter().find(|service| service.id == id) else {
        bail!("unknown service `{id}`");
    };

    let updated = config::Service {
        id: id.to_string(),
        name: name.unwrap_or_else(|| current.name.clone()),
        host: host.unwrap_or_else(|| current.host.clone()),
        run: run.unwrap_or_else(|| current.run.clone()),
        working_dir: working_dir.unwrap_or_else(|| current.working_dir.clone()),
        auto_start: auto_start.unwrap_or(current.auto_start),
    };

    match control_request(
        ControlCommand::Edit,
        Some(id.to_string()),
        Some(updated.clone()),
    ) {
        Ok(response) => print_snapshot(response_snapshot(response)?),
        Err(_) => {
            config::replace_service(&mut config, id, updated)?;
            let path = config::save(&config)?;
            println!("Edited service `{id}` in {}", path.display());
            Ok(())
        }
    }
}

fn remove(id: &str) -> Result<()> {
    match control_request(ControlCommand::Remove, Some(id.to_string()), None) {
        Ok(response) => print_snapshot(response_snapshot(response)?),
        Err(_) => {
            let mut config = config::load()?;
            config::remove_service(&mut config, id)?;
            let path = config::save(&config)?;
            println!("Removed service `{id}` from {}", path.display());
            Ok(())
        }
    }
}

fn log(id: &str) -> Result<()> {
    let response = control_request(ControlCommand::Log, Some(id.to_string()), None)?;
    if !response.ok {
        bail!(response
            .error
            .unwrap_or_else(|| "service log failed".to_string()));
    }

    print!("{}", response.log.unwrap_or_default());
    Ok(())
}

fn resolve_service_id(input: &str) -> Result<String> {
    let config = config::load()?;
    let matches = config
        .services
        .iter()
        .filter(|service| service.id == input || service.name == input || service.host == input)
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [] => Ok(input.to_string()),
        [service] => Ok(service.id.clone()),
        _ => bail!("ambiguous service `{input}`; use the service id"),
    }
}
