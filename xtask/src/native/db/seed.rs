//! Restore the native portion of scripts/seed_db.sh's URL-based seeding.
//! Resolve the native target explicitly; require psql and a migrated database.
//! Share xtask/seed_db.sql and the seed runner in crate::seed with Compose/cloud.
//! Use --no-psqlrc and ON_ERROR_STOP; preserve the fixture transaction.
//! Report failures without exposing credentials; do not start or reset the DB.

use anyhow::{Result, bail};

pub fn run() -> Result<()> {
    bail!("native db seed is not implemented yet")
}
