//! Check SQLx metadata freshness against an explicitly selected, migrated database.
//! Run cargo sqlx prepare --workspace --check -- --all-targets from project_root().
//! Preserve the existing CI command's exit status so stale metadata fails the check.
//! Do not rewrite metadata, apply migrations, reset a database, or start containers.

use anyhow::{Result, bail};

pub fn run() -> Result<()> {
    bail!("sqlx check is not implemented yet")
}
