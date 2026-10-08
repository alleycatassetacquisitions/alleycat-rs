use crate::env::{env_path, read_project_env};
use anyhow::{Result, bail};
use url::Url;

pub(super) fn setting(key: &str) -> Result<String> {
    let value = match std::env::var(key) {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) if env_path()?.exists() => {
            read_project_env()?.remove(key)
        }
        Err(std::env::VarError::NotPresent) => None,
        Err(_) => bail!("{key} must contain valid Unicode"),
    };
    value
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("Set {key} in .env or the shell environment"))
}

pub(super) fn database() -> Result<Url> {
    parse_database(&setting("REMOTE_DB_URL")?)
}

fn parse_database(value: &str) -> Result<Url> {
    let url = Url::parse(value).map_err(|_| anyhow::anyhow!("REMOTE_DB_URL is not a valid URL"))?;
    if !matches!(url.scheme(), "postgres" | "postgresql")
        || !url
            .host_str()
            .is_some_and(|h| h.ends_with(".ondigitalocean.com"))
        || url.path().trim_matches('/').is_empty()
        || url.fragment().is_some()
    {
        bail!("REMOTE_DB_URL must specify a PostgreSQL database on a DigitalOcean hostname");
    }
    Ok(url)
}

pub(super) fn deployment() -> Result<Url> {
    parse_deployment(&setting("REMOTE_DEPLOY_URL")?)
}

fn parse_deployment(value: &str) -> Result<Url> {
    let mut url =
        Url::parse(value).map_err(|_| anyhow::anyhow!("REMOTE_DEPLOY_URL is not a valid URL"))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        bail!(
            "REMOTE_DEPLOY_URL must be an HTTP(S) base URL without credentials, query, or fragment"
        );
    }
    if !url.path().ends_with('/') {
        url.set_path(&format!("{}/", url.path()));
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remote_host_must_be_a_real_do_subdomain() {
        assert!(
            parse_database("postgres://u:p@db.ondigitalocean.com:25060/defaultdb?sslmode=require")
                .is_ok()
        );
        for value in [
            "postgres://ondigitalocean.com:secret@localhost/db",
            "postgres://u:p@db.ondigitalocean.com.evil.test/db",
            "postgres://localhost/db",
            "postgres://db.ondigitalocean.com/",
            "https://db.ondigitalocean.com/db",
        ] {
            let error = parse_database(value).unwrap_err().to_string();
            assert!(!error.contains(value));
            assert!(!error.contains("secret"));
        }
    }
    #[test]
    fn deployment_paths_are_preserved_without_credentials() {
        let base = parse_deployment("https://example.com/api").unwrap();
        assert_eq!(
            base.join("health_check").unwrap().as_str(),
            "https://example.com/api/health_check"
        );
        assert!(parse_deployment("https://user:secret@example.com").is_err());
        assert!(parse_deployment("https://example.com?token=secret").is_err());
    }
}
