use crate::command::command_output;
use crate::env::{check_env, read_project_env};
use anyhow::{Result, bail};

pub fn run() -> Result<()> {
    let checks = [
        check_rust(),
        check_docker_cli(),
        check_docker_compose(),
        check_docker_running(),
        check_env_file(),
        check_env_keys(),
    ];

    println!("Alleycat local Compose stack\n");

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
    match command_output("rustc", &["--version"]) {
        Ok(version) => CheckResult {
            name: "Rust toolchain",
            outcome: CheckOutcome::Pass(version),
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

fn check_docker_compose() -> CheckResult {
    match command_output("docker", &["compose", "version"]) {
        Ok(version) => CheckResult {
            name: "Docker Compose",
            outcome: CheckOutcome::Pass(version),
        },
        Err(detail) => CheckResult {
            name: "Docker Compose",
            outcome: CheckOutcome::Fail {
                detail,
                fix: "Install Docker Compose and ensure `docker compose` is on PATH.",
            },
        },
    }
}

fn check_docker_running() -> CheckResult {
    match command_output("docker", &["info"]) {
        Ok(_) => CheckResult {
            name: "Docker",
            outcome: CheckOutcome::Pass("daemon running".to_owned()),
        },
        Err(detail) => CheckResult {
            name: "Docker",
            outcome: CheckOutcome::Fail {
                detail,
                fix: "Ensure Docker is running and `docker` is on PATH.",
            },
        },
    }
}

fn check_env_file() -> CheckResult {
    match check_env() {
        Ok(()) => CheckResult {
            name: "Environment File",
            outcome: CheckOutcome::Pass("environment file exists".to_owned()),
        },
        Err(error) => CheckResult {
            name: "Environment File",
            outcome: CheckOutcome::Fail {
                detail: format!("{error:#}"),
                fix: "Ensure the project's .env file exists and is accessible.",
            },
        },
    }
}

fn check_env_keys() -> CheckResult {
    const REQUIRED_KEYS: [&str; 3] = ["POSTGRES_USER", "POSTGRES_PASSWORD", "POSTGRES_DB"];

    let values = match read_project_env() {
        Ok(values) => values,
        Err(error) => {
            return CheckResult {
                name: "Environment Keys",
                outcome: CheckOutcome::Fail {
                    detail: format!("could not read .env: {error}"),
                    fix: "Check that `.env` exists and contains valid dotenv syntax.",
                },
            };
        }
    };

    let missing_or_empty: Vec<&str> = REQUIRED_KEYS
        .into_iter()
        .filter(|key| values.get(*key).is_none_or(|value| value.trim().is_empty()))
        .collect();

    if missing_or_empty.is_empty() {
        CheckResult {
            name: "Environment Keys",
            outcome: CheckOutcome::Pass("environment keys are set".to_owned()),
        }
    } else {
        CheckResult {
            name: "Environment Keys",
            outcome: CheckOutcome::Fail {
                detail: format!("missing or empty keys: {:?}", missing_or_empty),
                fix: "Set values for required environment variables. (see .env.example)",
            },
        }
    }
}
