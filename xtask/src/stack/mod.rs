use anyhow::Result;
use clap::Subcommand;

mod doctor;
mod start;

#[derive(Subcommand)]
pub enum Command {
    /// Check requirements for the local Compose stack.
    Doctor,
    /// Start the local Compose stack.
    Start,
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Doctor => doctor::run(),
        Command::Start => start::run(),
    }
}
