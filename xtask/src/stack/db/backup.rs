//! Replace scripts/backup_db.sh with a dump of the Compose Postgres database.
//! Execute pg_dump inside the postgres service using its configured credentials.
//! Write UTC-timestamped SQL under backups/db/ with private permissions.
//! Stream into a temporary file and finalize only after pg_dump succeeds.
//! Report the path and propagate failures so stack deletion cannot continue
//! after an incomplete backup. Resolve all paths from project_root().

use anyhow::{Result, bail};

pub fn run() -> Result<()> {
    bail!("stack db backup is not implemented yet")
}
