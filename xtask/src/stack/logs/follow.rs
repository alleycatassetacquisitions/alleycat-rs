use crate::project::project_root;
use anyhow::{Context, Result, bail};
use std::process::Command;

pub fn run() -> Result<()> {
    let status = Command::new("docker")
        .current_dir(project_root()?)
        .args([
            "compose",
            "logs",
            "--no-color",
            "--no-log-prefix",
            "-f",
            "app",
        ])
        .status()
        .context("could not follow Docker Compose logs")?;
    if !status.success() {
        bail!("Docker Compose logs failed with {status}");
    }
    Ok(())
}
