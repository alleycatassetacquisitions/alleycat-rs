use anyhow::{Context, Result, bail};
use clap::Subcommand;
use std::process::Command as Process;

mod db;
mod target;

#[derive(Subcommand)]
pub enum Command {
    /// Check the deployment's health using REMOTE_DEPLOY_URL (requires curl).
    Status,
    /// Manage the DigitalOcean database configured by REMOTE_DB_URL.
    Db {
        #[command(subcommand)]
        command: db::Command,
    },
}

pub fn run(command: Command) -> Result<()> {
    match command {
        Command::Db { command } => db::run(command),
        Command::Status => {
            let url = target::deployment()?.join("health_check")?;
            let output = Process::new("curl")
                .args([
                    "--silent",
                    "--show-error",
                    "--connect-timeout",
                    "10",
                    "--max-time",
                    "30",
                    "--output",
                    "/dev/null",
                    "--write-out",
                    "%{http_code}",
                    url.as_str(),
                ])
                .output()
                .context("could not run curl for deployment health check")?;
            if !output.status.success() {
                bail!("deployment health check could not connect");
            }
            let status = String::from_utf8_lossy(&output.stdout);
            if status != "200" {
                bail!("deployment health check returned HTTP {status}");
            }
            println!("Deployment healthy: {url} (HTTP 200)");
            Ok(())
        }
    }
}
