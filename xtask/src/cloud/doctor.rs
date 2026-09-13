//! Inspect the explicitly selected cloud target without changing shared state.
//! Start with target validation, required tools, connectivity, and migration
//! status. The broader checks below are follow-ups, not prerequisites for
//! porting reset. Fail on required-check errors without exposing credentials.

use anyhow::{Result, bail};

pub fn run() -> Result<()> {
    bail!("cloud doctor is not implemented yet")
}

// Suggested future cloud checks, roughly in implementation order:
//
// Tooling and authentication
// - `doctl` is installed and authenticated when the operation requires it.
// - `sqlx` is installed at a version compatible with SQLx 0.8 for migrations.
// - Git can identify the repository, current branch, and configured remote.
//
// Repository and target configuration
// - `spec.yaml` passes DigitalOcean App Platform schema validation.
// - Resolve the intended app ID and public origin without relying on memory.
// - Compare the live app spec with `spec.yaml` before applying changes.
// - DATABASE_URL is set without printing it and targets the intended
//   DigitalOcean database before inspecting or applying migrations.
//
// Remote state and safety
// - Report pending migrations without changing the remote database.
// - The public health check responds after deployment.
// - A database-backed endpoint responds after deployment.
// - Warn that the current public API is unauthenticated.
// - Confirm the backup and recovery policy before data becomes important.
// - Keep destructive database reset validation separate from routine deployment.
