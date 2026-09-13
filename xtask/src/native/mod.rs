//! Dispatch host-native workflows; keep database lifecycle work in `db`.

use anyhow::Result;
use clap::Subcommand;

mod db;
mod doctor;

#[derive(Subcommand)]
pub enum Command {
    /// Check requirements for host-native development (not implemented yet).
    Doctor,
    /// Manage the standalone native database (not implemented yet).
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
