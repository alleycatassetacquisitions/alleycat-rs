use ::anyhow::Result;
use clap::Subcommand;

mod backup;
mod migrate;
mod reset;
mod seed;

#[derive(Subcommand)]
pub enum Command {
    /// Run migrations.
    Migrate,
    /// Seed the database with dummy data.
    Seed,
    /// Backup the database.
    Backup,
    /// Reset the database to a clean state.
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
