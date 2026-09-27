//! Follow logs when no subcommand is supplied; dispatch explicit log backups.

use anyhow::Result;
use clap::Subcommand;

mod backup;
mod follow;

#[derive(Subcommand)]
pub enum Command {
    /// Export API logs to a timestamped JSONL file (not implemented yet).
    Backup,
}

pub fn run(command: Option<Command>) -> Result<()> {
    match command {
        None => follow::run(),
        Some(Command::Backup) => backup::run(),
    }
}
