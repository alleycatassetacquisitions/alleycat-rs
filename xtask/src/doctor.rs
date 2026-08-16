use anyhow::{Result, bail};
use std::process::Command as ProcessCommand;

pub fn run() -> Result<()> {
    let checks = [check_rust(), check_docker_cli()];

    println!("Alleycat development environment\n");

    let mut failure_count = 0;
    for check in checks {
        match check.outcome {
            CheckOutcome::Pass(detail) => {
                println!("[PASS] {}: {detail}", check.name);
            }
            CheckOutcome::Fail { detail, fix } => {
                failure_count += 1;
                println!("[FAIL] {}: {detail}", check.name);
                println!("       Fix: {fix}");
            }
        }
    }

    println!();
    if failure_count > 0 {
        bail!("{failure_count} required check(s) failed");
    }

    println!("All required checks passed.");
    Ok(())
}

struct CheckResult {
    name: &'static str,
    outcome: CheckOutcome,
}

enum CheckOutcome {
    Pass(String),
    Fail { detail: String, fix: &'static str },
}

fn check_rust() -> CheckResult {
    const EXPECTED_VERSION: &str = "rustc 1.95.0";

    match command_output("rustc", &["--version"]) {
        Ok(version) if version.starts_with(EXPECTED_VERSION) => CheckResult {
            name: "Rust toolchain",
            outcome: CheckOutcome::Pass(version),
        },
        Ok(version) => CheckResult {
            name: "Rust toolchain",
            outcome: CheckOutcome::Fail {
                detail: format!("expected {EXPECTED_VERSION}, found {version}"),
                fix: "Run `rustup toolchain install 1.95.0`, then retry from the repository root without a `+toolchain` override.",
            },
        },
        Err(detail) => CheckResult {
            name: "Rust toolchain",
            outcome: CheckOutcome::Fail {
                detail,
                fix: "Install Rustup, then run this command again from the repository root.",
            },
        },
    }
}

fn check_docker_cli() -> CheckResult {
    match command_output("docker", &["--version"]) {
        Ok(version) => CheckResult {
            name: "Docker CLI",
            outcome: CheckOutcome::Pass(version),
        },
        Err(detail) => CheckResult {
            name: "Docker CLI",
            outcome: CheckOutcome::Fail {
                detail,
                fix: "Install Docker Desktop or Docker Engine and ensure `docker` is on PATH.",
            },
        },
    }
}

fn command_output(program: &str, arguments: &[&str]) -> Result<String, String> {
    match ProcessCommand::new(program).args(arguments).output() {
        Ok(output) if output.status.success() => {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            if stderr.is_empty() {
                Err(format!("`{program}` exited with {}", output.status))
            } else {
                Err(stderr)
            }
        }
        Err(error) => Err(format!("could not run `{program}`: {error}")),
    }
}

// Suggested future checks, roughly in implementation order:
//
// Tooling
// - `docker compose version` reports a usable Compose plugin.
// - `docker info` confirms that the Docker daemon is running.
// - Node and npm meet the versions required by `web/package.json`.
// - `sqlx` is installed when migrations or SQLx metadata will be changed.
// - `psql` is installed when a host-side DATABASE_URL will be used for seeding.
//
// Repository configuration
// - The command is running inside the Alleycat repository.
// - `.env` exists when the selected workflow requires it.
// - `.env` contains POSTGRES_USER, POSTGRES_PASSWORD, and POSTGRES_DB.
// - POSTGRES_PASSWORD is safe to interpolate into the Compose database URL.
// - `docker compose config --quiet` accepts the resolved configuration.
//
// Local state
// - Ports 3000, 44100, 8000, and 5432 are free or owned by the expected service.
// - A container named `postgres` will not collide with native database setup.
// - The standalone native database is running when native tests require it.
// - Compose services are healthy when a stack command expects them to be running.
// - The API health check responds, followed by a database-backed readiness check.
//
// Safety and shared-environment checks
// - Classify DATABASE_URL as native, Compose, or remote before mutating data.
// - Warn when an inherited DATABASE_URL changes the target of `seed_db.sh`.
// - Confirm `doctl` is installed and authenticated before shared deployment commands.
// - Verify that the live DigitalOcean app and database match the intended target.
//
// As checks become dependent, add Warn and Skip outcomes. For example, skip
// Compose health checks when the Docker daemon check fails instead of reporting
// several redundant failures.
