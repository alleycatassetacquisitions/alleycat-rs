//! Replace the metadata-refresh portion of scripts/prepare_sqlx.sh.
//! Require a compatible SQLx CLI and an explicitly selected, migrated native DB.
//! Run cargo sqlx prepare --workspace -- --all-targets from project_root().
//! Refresh the checked .sqlx metadata and propagate subprocess failures.
//! Do not implicitly destroy/recreate the native DB; leave reset as an explicit task.
//! Keep database credentials out of logs and out of the parent process environment.

use anyhow::{Result, bail};

pub fn run() -> Result<()> {
    bail!("sqlx prepare is not implemented yet")
}
