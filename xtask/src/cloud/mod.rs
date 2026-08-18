use anyhow::Result;
use clap::Subcommand;

mod doctor;

#[derive(Subcommand)]
pub enum Command {
    /// Check requirements for the shared cloud environment.
    Doctor,
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Doctor => doctor::run(),
    }
}
