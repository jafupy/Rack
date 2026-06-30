mod hook;
mod hook_test;
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

#[derive(Subcommand)]
enum HookCommand {
    #[command(visible_alias = "ls")]
    List,
    Init {
        path: String,
    },
    #[command(visible_alias = "compile")]
    Build {
        path: Option<String>,
    },
    #[command(visible_alias = "install")]
    Deploy {
        path: Option<String>,
    },
    Test {
        path: Option<String>,
        #[arg(long)]
        hook: Option<String>,
        #[arg(long)]
        route: Option<String>,
    },
    #[command(visible_alias = "rm", visible_alias = "uninstall")]
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
        Some(Commands::List) => service::run(Some(service::Command::List)),
        Some(Commands::Start { id }) => service::run(Some(service::Command::Start { id })),
        Some(Commands::Stop { id }) => service::run(Some(service::Command::Stop { id })),
        Some(Commands::Restart { id }) => service::run(Some(service::Command::Restart { id })),
        Some(Commands::Remove { id }) => service::run(Some(service::Command::Remove { id })),
    }
}

fn run_hook(command: HookCommand) -> Result<()> {
    match command {
        HookCommand::List => hook::list(),
        HookCommand::Init { path } => hook::init(&path),
        HookCommand::Build { path } => hook::build(path.as_deref().unwrap_or(".")),
        HookCommand::Deploy { path } => hook::deploy(path.as_deref().unwrap_or(".")),
        HookCommand::Test { path, hook, route } => hook_test::run(
            path.as_deref().unwrap_or("."),
            hook.as_deref(),
            route.as_deref(),
        ),
        HookCommand::Remove { name } => hook::remove(&name),
    }
}
