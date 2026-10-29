//! Dispatch Compose database workflows; seeding already has an implementation.

use crate::database::{self, BackupArgs};
use anyhow::Result;
use clap::Subcommand;
use std::path::PathBuf;

mod reset;
mod seed;

#[derive(Subcommand)]
pub enum Command {
    /// Seed the database with dummy data.
    Seed,
    /// Delete local database data, rebuild, migrate, and seed the stack.
    Reset,
    /// Save the whole Compose database as a timestamped SQL file.
    Backup(BackupArgs),
    /// Restore a trusted SQL backup into an empty Compose database.
    Restore { file: PathBuf },
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Seed => seed::run(),
        Command::Reset => reset::run(),
        Command::Backup(args) => {
            database::backup(database::stack_client("pg_dump")?, args).map(|_| ())
        }
        Command::Restore { file } => database::restore(database::stack_client("psql")?, &file),
    }
}
