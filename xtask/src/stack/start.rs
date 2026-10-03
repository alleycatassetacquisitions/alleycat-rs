use crate::env::read_project_env;
use crate::project::project_root;
use anyhow::{Context, Result, bail};
use std::process::{Command, Stdio};

pub fn run() -> Result<()> {
    super::doctor::run()?;

    let project_root = project_root()?;

    let status = Command::new("docker")
        .args(["compose", "build"])
        .current_dir(project_root)
        .status()
        .context("could not run Docker Compose build")?;

    if !status.success() {
        bail!("Docker Compose build failed with {status}");
    }

    // Start only PostgreSQL so no migrations run before the reset policy check.
    let status = Command::new("docker")
        .args(["compose", "up", "-d", "--wait", "postgres"])
        .current_dir(project_root)
        .status()
        .context("could not start PostgreSQL for migration checks")?;
    if !status.success() {
        bail!("PostgreSQL startup failed with {status}");
    }

    super::migration_check::run()?;

    let _ = Command::new("docker")
        .args(["compose", "rm", "-f", "migrate"])
        .current_dir(project_root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    let status = Command::new("docker")
        .args(["compose", "up", "-d"])
        .current_dir(project_root)
        .status()
        .context("could not run Docker Compose")?;

    if !status.success() {
        bail!("Docker Compose failed with {status}");
    }

    let values = read_project_env()?;

    let app_port = values
        .get("APP_PORT")
        .filter(|port| !port.is_empty())
        .map_or("8000", String::as_str);

    println!("Local instance is starting.");
    println!("Health check: curl -i http://localhost:{app_port}/health_check");
    println!("API logs: cargo xtask stack logs");

    Ok(())
}
