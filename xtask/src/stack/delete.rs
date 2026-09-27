//! Replace scripts/delete_local_instance.sh for the selected Compose project.
//! Require the existing 'delete local instance' confirmation before changing state.
//! Back up app logs, then the database; abort teardown if either backup fails.
//! Only after both backups succeed, run compose down --volumes --remove-orphans.
//! Use project_root(), report backup paths, and propagate teardown failures.

use crate::project::project_root;
use anyhow::{Context, Result, bail};
use std::io::{self, Write};

pub fn run() -> Result<()> {
    // set script dir?
    let _project_root = project_root()?;
    let mut stderr = io::stderr().lock();

    let confirmation_phrase = "delete";

    eprintln!("This deletes the local Docker containers, network, and Postgres data volume.");
    eprintln!("The script will back up app logs and the database before deleting anything.");
    eprint!("Type 'delete' to confirm: ");
    stderr.flush()?;

    let mut input = String::new();

    let bytes_read = io::stdin()
        .read_line(&mut input)
        .context("could not read confirmation")?;

    if bytes_read == 0 || input.trim_end_matches(['\r', '\n']) != confirmation_phrase {
        bail!("Deletion canceled");
    }

    bail!("stack delete is not implemented yet")
}
