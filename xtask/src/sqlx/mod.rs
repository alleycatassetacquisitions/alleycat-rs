//! Dispatch SQLx metadata operations independently from destructive database reset.

use anyhow::Result;
use clap::Subcommand;

mod check;
mod prepare;

#[derive(Subcommand)]
pub enum Command {
    /// Refresh checked query metadata (not implemented yet).
    Prepare,
    /// Check query metadata without rewriting it (not implemented yet).
    Check,
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Prepare => prepare::run(),
        Command::Check => check::run(),
    }
}
