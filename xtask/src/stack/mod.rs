//! Dispatch Compose workflows; leave working startup and doctor behavior intact.

use anyhow::Result;
use clap::Subcommand;

mod db;
mod delete;
mod doctor;
mod logs;
mod start;

#[derive(Subcommand)]
pub enum Command {
    /// Check requirements for the local Compose stack.
    Doctor,
    /// Start the local Compose stack.
    Start,
    /// Back up and delete the local stack and its volume (not implemented yet).
    Delete,
    /// Follow or back up API logs (not implemented yet).
    Logs {
        #[command(subcommand)]
        command: Option<logs::Command>,
    },
    /// Database commands
    Db {
        #[command(subcommand)]
        command: db::Command,
    },
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Doctor => doctor::run(),
        Command::Start => start::run(),
        Command::Delete => delete::run(),
        Command::Logs { command } => logs::run(command),
        Command::Db { command } => db::run(command),
    }
}
