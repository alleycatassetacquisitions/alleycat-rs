//! Dispatch host-native workflows; keep database lifecycle work in `db`.

use anyhow::Result;
use clap::Subcommand;

mod db;

#[derive(Subcommand)]
pub enum Command {
    /// Manage the standalone native database (not implemented yet).
    Db {
        #[command(subcommand)]
        command: db::Command,
    },
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Db { command } => db::run(command),
    }
}
