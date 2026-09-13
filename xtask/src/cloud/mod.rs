//! Dispatch shared-cloud workflows without implicitly selecting a local target.

use anyhow::Result;
use clap::Subcommand;

mod db;
mod doctor;

#[derive(Subcommand)]
pub enum Command {
    /// Check requirements for the shared cloud environment (not implemented yet).
    Doctor,
    /// Manage an explicitly selected cloud database (not implemented yet).
    Db {
        #[command(subcommand)]
        command: db::Command,
    },
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Doctor => doctor::run(),
        Command::Db { command } => db::run(command),
    }
}
