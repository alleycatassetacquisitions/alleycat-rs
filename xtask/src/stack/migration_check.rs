use crate::project::project_root;
use anyhow::{Context, Result, bail};
use std::process::{Command, Stdio};

pub(super) fn run() -> Result<()> {
    let output = Command::new("docker")
        .args([
            "compose",
            "run",
            "--rm",
            "--no-deps",
            "-T",
            "--entrypoint",
            "sqlx",
            "-e",
            "NO_COLOR=1",
            "-e",
            "CLICOLOR=0",
            "migrate",
            "migrate",
            "info",
        ])
        .current_dir(project_root()?)
        .stderr(Stdio::inherit())
        .output()
        .context("could not run sqlx migrate info")?;
    if !output.status.success() {
        bail!(
            "SQLx migration check failed with {}; startup stopped",
            output.status
        );
    }
    if requires_reset(std::str::from_utf8(&output.stdout)?) {
        bail!(
            "Pending migrations require a database reset. Run `cargo xtask stack db reset`. This deletes all local database data, applies migrations, and seeds sample data."
        );
    }
    Ok(())
}

fn requires_reset(output: &str) -> bool {
    let has_status = |expected| {
        output.lines().any(|line| {
            line.split_whitespace()
                .next()
                .and_then(|entry| entry.split_once('/'))
                .is_some_and(|(version, status)| {
                    version.parse::<i64>().is_ok() && status == expected
                })
        })
    };
    has_status("installed") && has_status("pending")
}

#[cfg(test)]
mod tests {
    use super::requires_reset;

    #[test]
    fn installed_and_pending_require_reset() {
        assert!(requires_reset("1/installed first\n2/pending second\n"));
    }

    #[test]
    fn only_pending_can_initialize() {
        assert!(!requires_reset("1/pending first\n2/pending second\n"));
    }

    #[test]
    fn only_installed_can_start() {
        assert!(!requires_reset("1/installed first\n2/installed second\n"));
        assert!(!requires_reset(""));
    }

    #[test]
    fn descriptions_do_not_count_as_statuses() {
        assert!(!requires_reset(
            "1/installed add pending jobs\n2/installed add /pending route\n"
        ));
        assert!(!requires_reset(
            "1/pending add installed flag\n2/pending add /installed route\n"
        ));
        assert!(requires_reset(
            "1/installed (different checksum) first\n2/pending second\n"
        ));
    }
}
