//! Replace scripts/kill_db.sh: force-remove the standalone postgres container
//! and its anonymous volumes after checking the target and confirming deletion.
//! Keep this separate from Compose teardown; do not remove unrelated containers.
//! Handle an absent container deliberately and propagate real Docker failures.
//! Allow reset to reuse the operation after its own confirmation without prompting twice.

use anyhow::{Result, bail};

pub fn run() -> Result<()> {
    bail!("native db delete is not implemented yet")
}
