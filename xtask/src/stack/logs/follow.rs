//! Replace scripts/view_logs.sh by following the Compose app service logs.
//! Run from project_root() with the same environment handling as other stack tasks.
//! Pipe logs without color or log prefixes through Bunyan when it is installed;
//! otherwise explain the fallback and stream raw Compose logs.
//! Stream rather than buffer, handle interruption, and propagate process failures.

use anyhow::{Result, bail};

pub fn run() -> Result<()> {
    bail!("stack logs is not implemented yet")
}
