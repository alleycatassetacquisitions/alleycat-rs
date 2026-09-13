//! Restore explicit remote URL-based fixture seeding from scripts/seed_db.sh.
//! Require and validate DATABASE_URL; identify the target without printing credentials.
//! Require deliberate opt-in to insert/update fixtures in a disposable cloud database.
//! Share xtask/seed_db.sql and crate::seed's runner instead of duplicating SQL.
//! Use psql with --no-psqlrc and ON_ERROR_STOP, and propagate transaction failures.

use anyhow::{Result, bail};

pub fn run() -> Result<()> {
    bail!("cloud db seed is not implemented yet")
}
