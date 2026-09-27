//! Replace scripts/reset_remote_db.sh for disposable DigitalOcean databases only.
//! Parse the URL and validate its hostname; never use a substring check of the whole URL.
//! Require confirmation of the database name, preserving CONFIRM_REMOTE_DB_RESET
//! as an explicit noninteractive opt-in. Never fall back to a local dotenv target.
//! Drop only the explicit application object inventory and SQLx's migration ledger.
//! Keep the script inventory synchronized with migrations while it remains in use,
//! and update AGENTS.md when ownership moves here. Never blanket-drop public.
//! Reapply migrations, show status, and stop on any error. Never invoke this in
//! routine verification, doctor checks, or deployment.

use anyhow::{Result, bail};

pub fn run() -> Result<()> {
    bail!("cloud db reset is not implemented yet")
}
