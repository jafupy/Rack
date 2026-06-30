mod hook;
mod service;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rack")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(visible_alias = "services")]
    Service {
        #[command(subcommand)]
        command: Option<service::Command>,
    },
    Hook {
        #[command(subcommand)]
        command: hook::Command,
    },
    #[command(visible_alias = "ls")]
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
    #[command(visible_alias = "rm")]
    Remove {
        id: String,
    },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    match Cli::parse().command {
        None => service::run(None),
        Some(Commands::Service { command }) => service::run(command),
        Some(Commands::Hook { command }) => hook::run(command),
        Some(Commands::List) => service::run(Some(service::Command::List)),
        Some(Commands::Start { id }) => service::run(Some(service::Command::Start { id })),
        Some(Commands::Stop { id }) => service::run(Some(service::Command::Stop { id })),
        Some(Commands::Restart { id }) => service::run(Some(service::Command::Restart { id })),
        Some(Commands::Remove { id }) => service::run(Some(service::Command::Remove { id })),
    }
}
