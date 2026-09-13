//! Replace scripts/reset_db.sh by composing native deletion and initialization.
//! This workflow destroys the standalone Postgres container and its volumes.
//! Identify the native target and obtain explicit confirmation before deletion.
//! Abort if deletion fails; then initialize the container, role, DB, and migrations.
//! Do not interpret SKIP_DOCKER as permission to destroy an external database.

use anyhow::{Result, bail};

pub fn run() -> Result<()> {
    bail!("native db reset is not implemented yet")
}
