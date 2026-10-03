//! Dispatch Compose database workflows; seeding already has an implementation.

use anyhow::Result;
use clap::Subcommand;

mod backup;
mod reset;
mod seed;

#[derive(Subcommand)]
pub enum Command {
    /// Seed the database with dummy data.
    Seed,
    /// Delete local database data, rebuild, migrate, and seed the stack.
    Reset,
    /// Back up the database (not implemented yet).
    Backup,
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Seed => seed::run(),
        Command::Reset => reset::run(),
        Command::Backup => backup::run(),
    }
}
