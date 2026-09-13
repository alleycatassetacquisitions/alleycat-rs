//! Apply pending migrations from the repository migrations directory.
//! Resolve and validate the native target using the same settings as initialization.
//! Require compatible SQLx CLI and an existing reachable database.
//! Run migrations from project_root() and propagate the subprocess exit status.
//! Do not recreate the database or implicitly target a shared cloud database.

use anyhow::{Result, bail};

pub fn run() -> Result<()> {
    bail!("native db migrate is not implemented yet")
}
