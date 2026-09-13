//! Replace scripts/backup_logs.sh by exporting Compose app service logs.
//! Use --no-color and --no-log-prefix; do not follow the stream.
//! Write a UTC-timestamped JSONL file under backups/logs/ with private permissions.
//! Write to a temporary file and finalize only after Compose exits successfully.
//! Report the path and allow stack deletion to reuse this operation; failures
//! must prevent deletion rather than leave a partial file marked as a backup.

use anyhow::{Result, bail};

pub fn run() -> Result<()> {
    bail!("stack logs backup is not implemented yet")
}
