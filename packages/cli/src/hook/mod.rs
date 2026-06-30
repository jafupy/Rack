mod build;
mod common;
mod deploy;
mod init;
mod list;
mod remove;
mod test;

use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum Command {
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

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::List => list::run(),
        Command::Init { path } => init::run(&path),
        Command::Build { path } => build::run(path.as_deref().unwrap_or(".")),
        Command::Deploy { path } => deploy::run(path.as_deref().unwrap_or(".")),
        Command::Test { path, hook, route } => test::run(
            path.as_deref().unwrap_or("."),
            hook.as_deref(),
            route.as_deref(),
        ),
        Command::Remove { name } => remove::run(&name),
    }
}
