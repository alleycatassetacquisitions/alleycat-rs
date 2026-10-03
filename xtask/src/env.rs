use crate::project::project_root;
use anyhow::{Context, Result, anyhow, bail};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub(crate) fn env_path() -> Result<PathBuf> {
    Ok(project_root()?.join(".env"))
}

pub(crate) fn check_env() -> Result<()> {
    if !env_path()?.try_exists().context("could not inspect .env")? {
        bail!("Missing .env. Create it with: cp .env.example .env");
    }
    Ok(())
}

pub(crate) fn read_project_env() -> Result<HashMap<String, String>> {
    read_env_file(env_path()?)
}

fn read_env_file(path: impl AsRef<Path>) -> Result<HashMap<String, String>> {
    dotenvy::from_path_iter(path)
        .and_then(|values| values.collect())
        // Raw dotenv errors can include credentials from the offending line.
        .map_err(|_| anyhow!("could not read .env; check its syntax and permissions"))
}

#[cfg(test)]
mod tests {
    use super::read_env_file;

    #[test]
    fn malformed_credentials_are_not_printed() {
        let path = std::env::temp_dir().join(format!("alleycat-env-{}.env", std::process::id()));
        std::fs::write(&path, "POSTGRES_PASSWORD=\"SYNTHETIC_SECRET\n").unwrap();
        let result = read_env_file(&path);
        std::fs::remove_file(path).unwrap();

        let error = format!("{:?}", result.unwrap_err());
        assert!(!error.contains("SYNTHETIC_SECRET"));
    }
}
