//! Inspect native prerequisites and database state without changing them.
//! Implement the checks below as needed by the native workflows; report fixes
//! and fail when a required check fails. Do not install tools or reset data.

use anyhow::{Result, bail};

pub fn run() -> Result<()> {
    bail!("native doctor is not implemented yet")
}

// Suggested future native-development checks, roughly in implementation order:
//
// Tooling
// - Report the active Rust toolchain selected by `rust-toolchain.toml`.
// - `sqlx` is installed at a version compatible with SQLx 0.8.
// - The Docker CLI is installed and the Docker daemon is running.
// - `psql` is installed when native sample data will be seeded through DATABASE_URL.
//
// Repository configuration
// - The command is running inside the Alleycat repository.
// - The migrations directory and checked SQLx metadata are present.
//
// Local state and safety
// - Port 5432 is free or belongs to the expected standalone Postgres container.
// - A container named `postgres` will not collide with unrelated user data.
// - The standalone Postgres container is healthy when a command requires it.
// - The application role and database exist and all migrations are applied.
// - DATABASE_URL resolves to the expected native database before mutating data.
// - Show test databases matched by the cleanup pattern before dropping them.
