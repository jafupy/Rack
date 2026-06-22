use anyhow::{bail, Result};
use rack_core::config;

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
        [scope, command] if service_scope(scope) && command == "list" => list_services(),
        [scope] if service_scope(scope) => list_services(),
        [scope, command] if service_scope(scope) => {
            bail!("unsupported service command `{command}`; try `rack service list`")
        }
        [command, ..] => bail!("unsupported command `{command}`; try `rack help`"),
    }
}

fn service_scope(value: &str) -> bool {
    value == "service" || value == "services"
}

fn list_services() -> Result<()> {
    let config = config::load()?;

    if config.services.is_empty() {
        println!("No services configured");
        return Ok(());
    }

    for service in config.services {
        let auto_start = if service.auto_start {
            " auto-start"
        } else {
            ""
        };
        println!(
            "{}\t{}\thttp://{}.localhost\t{}{}",
            service.id, service.name, service.host, service.run, auto_start
        );
    }

    Ok(())
}

fn print_help() {
    println!(
        "Rack\n\nUsage:\n  rack service list\n  rack services\n\nCommands:\n  service list   List configured services"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_service_scopes() {
        assert!(service_scope("service"));
        assert!(service_scope("services"));
        assert!(!service_scope("server"));
    }
}
