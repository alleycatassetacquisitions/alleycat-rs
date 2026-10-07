mod cloud;
mod command;
mod docs;
mod env;
mod native;
mod project;
mod seed;
mod sqlx;
mod stack;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate documentation from the repository's migrations.
    Docs {
        #[command(subcommand)]
        command: docs::Command,
    },
    /// Commands for host-native development.
    Native {
        #[command(subcommand)]
        command: native::Command,
    },
    /// Commands for managing local Compose deployments.
    Stack {
        #[command(subcommand)]
        command: stack::Command,
    },
    /// Commands for managing the shared cloud environment.
    Cloud {
        #[command(subcommand)]
        command: cloud::Command,
    },
    /// Commands for maintaining SQLx query metadata (not implemented yet).
    Sqlx {
        #[command(subcommand)]
        command: sqlx::Command,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Docs { command } => docs::run(command),
        Command::Native { command } => native::run(command),
        Command::Stack { command } => stack::run(command),
        Command::Cloud { command } => cloud::run(command),
        Command::Sqlx { command } => sqlx::run(command),
    }
}
