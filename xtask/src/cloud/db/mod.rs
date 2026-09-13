//! Dispatch cloud database operations; each implementation must require an explicit target.

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
    /// Seed the database with dummy data (not implemented yet).
    Seed,
    /// Back up the database (not implemented yet).
    Backup,
    /// Reset a disposable database (not implemented yet).
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
