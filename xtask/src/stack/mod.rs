use anyhow::Result;
use clap::Subcommand;

mod doctor;

#[derive(Subcommand)]
pub enum Command {
    /// Check requirements for the local Compose stack.
    Doctor,
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Doctor => doctor::run(),
    }
}
