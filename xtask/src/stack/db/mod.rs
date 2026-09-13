//! Dispatch Compose database workflows; seeding already has an implementation.

use anyhow::Result;
use clap::Subcommand;

mod backup;
mod migrate;
mod reset;
mod seed;

#[derive(Subcommand)]
pub enum Command {
    /// Run migrations (not implemented yet).
    Migrate,
    /// Seed the database with dummy data.
    Seed,
    /// Back up the database (not implemented yet).
    Backup,
    /// Reset the database to a clean state (not implemented yet).
    Reset,
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Migrate => migrate::run(),
        Command::Seed => seed::run(),
        Command::Backup => backup::run(),
        Command::Reset => reset::run(),
    }
}
