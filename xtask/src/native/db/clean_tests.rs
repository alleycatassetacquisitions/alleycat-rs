//! Replace scripts/clean_test_dbs.sh on the explicitly selected native server.
//! List non-template databases with the literal alleycat_test_ prefix.
//! Escape LIKE underscores or use a literal-prefix comparison; quote SQL identifiers.
//! Show the exact targets and confirm before terminating their sessions and dropping them.
//! Never select the application DB or unrelated databases; propagate SQL errors.

use anyhow::{Result, bail};

pub fn run() -> Result<()> {
    bail!("native db clean-tests is not implemented yet")
}
