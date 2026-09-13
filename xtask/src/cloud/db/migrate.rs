//! Apply pending migrations to an explicitly selected shared cloud database.
//! Require DATABASE_URL, validate its parsed target, and avoid a local dotenv fallback.
//! Use compatible SQLx CLI from project_root() and preserve TLS requirements.
//! Show migration status and propagate failures without exposing credentials.
//! Do not reset, recreate, or seed the database as part of migration.

use anyhow::{Result, bail};

pub fn run() -> Result<()> {
    bail!("cloud db migrate is not implemented yet")
}
