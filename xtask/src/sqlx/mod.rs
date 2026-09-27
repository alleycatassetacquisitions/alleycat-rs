//! Dispatch SQLx metadata operations independently from destructive database reset.

use anyhow::Result;
use clap::Subcommand;

mod prepare;

#[derive(Subcommand)]
pub enum Command {
    /// Refresh checked query metadata (not implemented yet).
    Prepare,
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Prepare => prepare::run(),
    }
}
