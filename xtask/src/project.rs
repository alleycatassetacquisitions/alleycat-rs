use anyhow::{Context, Result};
use std::path::Path;

pub(crate) fn project_root() -> Result<&'static Path> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .context("xtask should be inside the repository root")
}
