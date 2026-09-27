//! Dispatch shared-cloud workflows without implicitly selecting a local target.

use anyhow::Result;
use clap::Subcommand;

mod db;

#[derive(Subcommand)]
pub enum Command {
    /// Manage an explicitly selected cloud database (not implemented yet).
    Db {
        #[command(subcommand)]
        command: db::Command,
    },
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Db { command } => db::run(command),
    }
}
