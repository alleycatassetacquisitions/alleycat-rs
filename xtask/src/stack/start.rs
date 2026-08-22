use crate::env::read_env_file;
use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::{Command, Stdio};

pub fn run() -> Result<()> {
    super::doctor::run()?;

    let project_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("xtask should be inside the repository root")?;

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

    let values = read_env_file(project_root.join(".env"))?;

    let app_port = values
        .get("APP_PORT")
        .filter(|port| !port.is_empty())
        .map_or("8000", String::as_str);

    println!("Local instance is starting.");
    println!("Health check: curl -i http://localhost:{app_port}/health_check");
    println!("API logs: cargo xtask stack logs");

    Ok(())
}
