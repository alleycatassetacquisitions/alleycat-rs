use super::target;
use crate::project::project_root;
use anyhow::{Context, Result, bail};
use clap::Subcommand;
use std::{
    io::{self, Write},
    process::{Command as Process, Stdio},
};
use url::Url;

const IMAGE: &str = "alleycat-remote-db-tools";

#[derive(Subcommand)]
pub enum Command {
    /// Apply pending migrations to REMOTE_DB_URL without resetting data.
    Migrate,
    /// Show applied and pending remote migrations.
    Info,
    /// Insert sample data into the remote database (requires confirmation).
    Seed,
    /// Delete all application data and reapply migrations (requires confirmation).
    Reset,
}

pub fn run(command: Command) -> Result<()> {
    let url = target::database()?;
    println!("Remote database: {}{}", url.host_str().unwrap(), url.path());
    match command {
        Command::Reset => confirm(
            &url,
            "permanently delete all application data",
            Some("CONFIRM_REMOTE_DB_RESET"),
        )?,
        Command::Seed => confirm(
            &url,
            "insert or update sample data",
            Some("CONFIRM_REMOTE_DB_SEED"),
        )?,
        _ => (),
    }
    build_image()?;
    match command {
        Command::Migrate => {
            migrate(&url, "run")?;
            migrate(&url, "info")
        }
        Command::Info => migrate(&url, "info"),
        Command::Seed => sql_file(&url, "/app/seed_db.sql"),
        Command::Reset => {
            sql_file(&url, "/app/reset_remote_db.sql")?;
            migrate(&url, "run")?;
            migrate(&url, "info")
        }
    }
}

fn confirm(url: &Url, action: &str, opt_in: Option<&str>) -> Result<()> {
    let name = url.path().trim_start_matches('/');
    if opt_in
        .and_then(|key| std::env::var(key).ok())
        .is_some_and(|v| v == name)
    {
        return Ok(());
    }
    eprintln!("This will {action} in remote database '{name}'.");
    eprint!("Type '{name}' to continue: ");
    io::stderr().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    if input.trim_end_matches(['\r', '\n']) != name {
        bail!("Remote operation canceled");
    }
    Ok(())
}

fn build_image() -> Result<()> {
    let status = Process::new("docker")
        .args(["build", "-f", "Dockerfile.migrations", "-t", IMAGE, "."])
        .current_dir(project_root()?)
        .status()
        .context("could not build remote database tools; ensure Docker is running")?;
    if !status.success() {
        bail!("remote database tool image build failed");
    }
    Ok(())
}

fn migrate(url: &Url, action: &str) -> Result<()> {
    execute(url, "sqlx", &["migrate", action, "--no-dotenv"])
}

fn sql_file(url: &Url, file: &str) -> Result<()> {
    execute(
        url,
        "sh",
        &[
            "-c",
            "exec psql --dbname \"$DATABASE_URL\" --no-psqlrc --set ON_ERROR_STOP=1 --file \"$1\"",
            "sh",
            file,
        ],
    )
}

fn execute(url: &Url, entrypoint: &str, args: &[&str]) -> Result<()> {
    // Pass the secret through the child environment, never Docker CLI arguments.
    let output = Process::new("docker")
        .args([
            "run",
            "--rm",
            "--env",
            "DATABASE_URL",
            "--entrypoint",
            entrypoint,
            IMAGE,
        ])
        .args(args)
        .env("DATABASE_URL", url.as_str())
        .stdin(Stdio::null())
        .output()
        .context("could not run remote database tools")?;
    // Database/client errors can include connection credentials; suppress stderr.
    if !output.status.success() {
        bail!(
            "remote database command failed ({}); verify connectivity, credentials, and migration compatibility; raw output withheld to protect credentials",
            output.status
        );
    }
    print!(
        "{}",
        String::from_utf8_lossy(&output.stdout).replace(url.as_str(), "[redacted database URL]")
    );
    Ok(())
}
