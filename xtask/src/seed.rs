use crate::project::project_root;
use anyhow::{Context, Result, bail};
use std::fs::File;
use std::process::{Command, Stdio};

// pub(crate) fn seed_via_url(database_url: &str) -> Result<()> {
//     let mut command = Command::new("psql");
//     command
//         .arg(database_url)
//         .args(["--no-psqlrc", "--set", "ON_ERROR_STOP=1", "--file=-"]);

//     run_seed(command)
// }

pub(crate) fn seed_via_compose() -> Result<()> {
    let mut command = Command::new("docker");
    command
        .args([
            "compose",
            "exec",
            "-T",
            "postgres",
            "sh",
            "-c",
            r#"PGPASSWORD="$POSTGRES_PASSWORD" psql -h localhost -U "$POSTGRES_USER" "$POSTGRES_DB" --no-psqlrc --set ON_ERROR_STOP=1 --file=-"#,
        ])
        .current_dir(project_root()?);

    run_seed(command)
}

fn run_seed(mut command: Command) -> Result<()> {
    let seed_path = project_root()?.join("xtask/seed_db.sql");
    let seed_file = File::open(&seed_path)
        .with_context(|| format!("could not open {}", seed_path.display()))?;

    let status = command
        .stdin(Stdio::from(seed_file))
        .status()
        .context("could not run database seed command")?;

    if !status.success() {
        bail!("database seed failed with {status}");
    }

    Ok(())
}
