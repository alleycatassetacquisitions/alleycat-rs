//! Dispatch cloud database operations; each implementation must require an explicit target.

use anyhow::Result;
use clap::Subcommand;

mod reset;
mod seed;

#[derive(Subcommand)]
pub enum Command {
    /// Seed the database with dummy data (not implemented yet).
    Seed,
    /// Reset a disposable database (not implemented yet).
    Reset,
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Seed => seed::run(),
        Command::Reset => reset::run(),
    }
}
