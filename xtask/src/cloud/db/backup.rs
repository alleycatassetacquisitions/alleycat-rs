//! Back up an explicitly selected cloud database using a compatible pg_dump.
//! Validate DATABASE_URL, preserve TLS options, and avoid printing credentials.
//! Write a timestamped SQL dump under backups/db/ with private permissions.
//! Use a temporary file and finalize it only after pg_dump succeeds.
//! Distinguish the remote target in the filename/output without leaking secrets;
//! never alter the database and report failed or incomplete backups as errors.

use anyhow::{Result, bail};

pub fn run() -> Result<()> {
    bail!("cloud db backup is not implemented yet")
}
