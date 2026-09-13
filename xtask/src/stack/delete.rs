//! Replace scripts/delete_local_instance.sh for the selected Compose project.
//! Require the existing 'delete local instance' confirmation before changing state.
//! Back up app logs, then the database; abort teardown if either backup fails.
//! Only after both backups succeed, run compose down --volumes --remove-orphans.
//! Use project_root(), report backup paths, and propagate teardown failures.

use anyhow::{Result, bail};

pub fn run() -> Result<()> {
    bail!("stack delete is not implemented yet")
}
