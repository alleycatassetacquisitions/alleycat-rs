use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

pub(crate) fn read_env_file(
    path: impl AsRef<Path>,
) -> Result<HashMap<String, String>, dotenvy::Error> {
    dotenvy::from_path_iter(path)?.collect()
}
