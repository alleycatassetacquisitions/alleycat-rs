use crate::project::project_root;
use anyhow::{Context, Result, bail};
use std::io::{self, Write};
use std::process::Command;

pub fn run() -> Result<()> {
    super::super::doctor::run()?;

    eprintln!(
        "This permanently deletes all local Compose database data and replaces it with sample data."
    );
    eprintln!("The local stack will be stopped. No backup will be made.");
    eprint!("Type 'reset' to confirm: ");
    io::stderr().flush()?;

    let mut input = String::new();
    let bytes_read = io::stdin()
        .read_line(&mut input)
        .context("could not read reset confirmation")?;
    if bytes_read == 0 || input.trim_end_matches(['\r', '\n']) != "reset" {
        bail!("Database reset canceled");
    }

    let status = Command::new("docker")
        .args(["compose", "down", "--volumes", "--remove-orphans"])
        .current_dir(project_root()?)
        .status()
        .context("could not run Docker Compose teardown")?;
    if !status.success() {
        bail!("Docker Compose teardown failed with {status}");
    }

    super::super::start::run().context("database was reset, but stack startup failed")?;
    super::seed::run().context("database was reset, but seeding failed")?;

    println!("Local database reset complete: migrations applied and sample data seeded.");
    Ok(())
}
