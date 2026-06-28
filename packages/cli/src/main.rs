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
        command: HookCommand,
    },
}

#[derive(Subcommand)]
enum HookCommand {
    #[command(visible_alias = "ls")]
    List,
    Init {
        path: String,
    },
    Build {
        path: Option<String>,
    },
    Deploy {
        path: Option<String>,
    },
    #[command(visible_alias = "rm")]
    Remove {
        name: String,
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
        Some(Commands::Hook { command }) => run_hook(command),
    }
}

fn run_hook(command: HookCommand) -> Result<()> {
    match command {
        HookCommand::List => hook::list(),
        HookCommand::Init { path } => hook::init(&path),
        HookCommand::Build { path } => hook::build(path.as_deref().unwrap_or(".")),
        HookCommand::Deploy { path } => hook::deploy(path.as_deref().unwrap_or(".")),
        HookCommand::Remove { name } => hook::remove(&name),
    }
}
