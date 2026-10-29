//! Plain PostgreSQL backups shared by the cloud and Compose commands.

use crate::project::project_root;
use anyhow::{Context, Result, bail};
use clap::Args;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use url::Url;

#[derive(Args)]
pub struct BackupArgs {
    /// Destination directory, relative to the repository root unless absolute.
    #[arg(long, default_value = "backups/db")]
    pub output: PathBuf,
}

pub fn cloud_client(url: &Url, program: &str) -> Command {
    let mut command = Command::new("docker");
    command
        .args([
            "run",
            "--rm",
            "-i",
            "--env",
            "DATABASE_URL",
            "postgres:18",
            "sh",
            "-c",
            "exec \"$@\" --dbname \"$DATABASE_URL\"",
            "sh",
            program,
        ])
        .env("DATABASE_URL", url.as_str());
    command
}

pub fn stack_client(program: &str) -> Result<Command> {
    let mut command = Command::new("docker");
    command.args([
        "compose", "exec", "-T", "postgres", "sh", "-c",
        "export PGPASSWORD=\"$POSTGRES_PASSWORD\"; exec \"$@\" --host localhost --username \"$POSTGRES_USER\" --dbname \"$POSTGRES_DB\"",
        "sh", program,
    ]).current_dir(project_root()?);
    Ok(command)
}

struct PartialBackup(PathBuf);
impl Drop for PartialBackup {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

pub fn backup(mut command: Command, args: BackupArgs) -> Result<PathBuf> {
    let directory = project_root()?.join(args.output);
    fs::create_dir_all(&directory).context("could not create backup directory")?;
    let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%S%.9fZ").to_string();
    let path = directory.join(format!("alleycat-{timestamp}.sql"));
    let partial_path = path.with_extension("sql.partial");
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&partial_path)
        .context("could not create private backup file")?;
    let partial = PartialBackup(partial_path);
    let revision = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(project_root()?)
        .output()
        .context("could not read checkout revision")?;
    if !revision.status.success() {
        bail!("could not read checkout revision for backup metadata");
    }
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(project_root()?)
        .output()?;
    if !status.status.success() {
        bail!("could not read checkout status for backup metadata");
    }
    writeln!(
        file,
        "-- Alleycat database backup: {timestamp}\n-- Operator checkout: {}{}\n-- Server and pg_dump versions follow; applied versions/checksums are in _sqlx_migrations.\n-- Ownership and grants are omitted; restored objects belong to the restoring role.\n",
        String::from_utf8_lossy(&revision.stdout).trim(),
        if status.stdout.is_empty() {
            ""
        } else {
            " (with local changes)"
        }
    )?;
    let output = command
        .args([
            "--format=plain",
            "--no-owner",
            "--no-privileges",
            "--no-tablespaces",
            "--no-password",
        ])
        .stdin(Stdio::null())
        .stdout(file.try_clone()?)
        .stderr(Stdio::piped())
        .output()
        .context("could not run pg_dump through Docker")?;
    if !output.status.success() {
        bail!(
            "pg_dump failed ({}); no completed backup saved. Check connectivity, credentials, permissions and PostgreSQL client/server versions. Raw output withheld to protect credentials.",
            output.status
        );
    }
    file.sync_all()?;
    // Publishing a hard link is atomic and, unlike rename, cannot overwrite an
    // existing backup. Both names are on the same filesystem.
    fs::hard_link(&partial.0, &path).context("could not publish completed backup")?;
    if !output.stderr.is_empty() {
        eprintln!(
            "pg_dump completed with diagnostic output, withheld to protect credentials. Verify this backup with a recovery test before relying on it."
        );
    }
    println!("Backup saved: {}", path.display());
    Ok(path)
}

pub fn restore(mut command: Command, path: &Path) -> Result<()> {
    let path = project_root()?.join(path);
    let file = File::open(&path).context("could not open SQL backup")?;
    if file.metadata()?.len() == 0 {
        bail!("SQL backup is empty");
    }
    // The guard and dump run in the same psql transaction. No --clean, database
    // creation, migration or seeding step is part of recovery.
    let status = command
        .args([
            "--no-psqlrc",
            "--no-password",
            "--set=ON_ERROR_STOP=1",
            "--single-transaction",
            "--command",
            include_str!("database/empty_target.sql"),
            "--file=-",
        ])
        .stdin(file)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .context("could not run psql through Docker")?;
    if !status.success() {
        bail!(
            "Restore failed ({status}); SQL errors roll back the restore transaction. The target must be empty. Check connectivity, credentials, permissions and backup/server compatibility. Raw output withheld to protect credentials."
        );
    }
    println!("Database restored from {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests;
