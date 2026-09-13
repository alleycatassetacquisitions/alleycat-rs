//! Reset the application database in the selected local Compose project.
//! Decide and document schema/database reset semantics before implementing this;
//! full stack/volume deletion belongs to stack delete.
//! Identify the target and confirm the destructive action, require a successful
//! backup, and coordinate API access before changing the database.
//! Reapply migrations and report status; do not silently add sample data.
//! Stop on errors and never fall back to a native or cloud DATABASE_URL.

use anyhow::{Result, bail};

pub fn run() -> Result<()> {
    bail!("stack db reset is not implemented yet")
}
