mod proto;

use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum Command {
    /// Generate the self-hosted device API reference directly from .proto files.
    Proto {
        /// Fail if the generated reference is stale; do not rewrite it.
        #[arg(long)]
        check: bool,
    },
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Proto { check } => proto::generate(check),
    }
}
