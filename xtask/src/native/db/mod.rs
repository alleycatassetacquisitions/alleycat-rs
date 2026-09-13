//! Dispatch standalone database workflows that replace the native shell scripts.

use anyhow::Result;
use clap::Subcommand;

mod clean_tests;
mod delete;
mod init;
mod migrate;
mod reset;
mod seed;

#[derive(Subcommand)]
pub enum Command {
    /// Initialize standalone Postgres and apply migrations (not implemented yet).
    Init,
    /// Apply pending migrations (not implemented yet).
    Migrate,
    /// Seed the native database (not implemented yet).
    Seed,
    /// Recreate the native database container (not implemented yet).
    Reset,
    /// Remove the native database container and volumes (not implemented yet).
    Delete,
    /// Remove leftover test databases (not implemented yet).
    CleanTests,
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Init => init::run(),
        Command::Migrate => migrate::run(),
        Command::Seed => seed::run(),
        Command::Reset => reset::run(),
        Command::Delete => delete::run(),
        Command::CleanTests => clean_tests::run(),
    }
}
