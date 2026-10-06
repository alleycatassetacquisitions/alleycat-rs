use actix_web::{HttpResponse, web};

/// Deployment metadata captured once, before the server starts.
#[derive(serde::Serialize)]
pub struct VersionInfo {
    commit: Option<String>,
    commit_url: Option<String>,
}

impl VersionInfo {
    pub fn from_environment() -> Self {
        Self::new(
            std::env::var("DEPLOY_COMMIT_SHA").ok(),
            std::env::var("DEPLOY_REPOSITORY_URL").ok(),
        )
    }

    fn new(commit: Option<String>, repository_url: Option<String>) -> Self {
        let commit = commit.filter(|value| !value.trim().is_empty());
        let repository_url = repository_url.filter(|value| !value.trim().is_empty());
        let commit_url = commit.as_ref().zip(repository_url).map(|(sha, url)| {
            format!("{}/commit/{}", url.trim().trim_end_matches('/'), sha.trim())
        });
        Self { commit, commit_url }
    }
}

pub async fn get_version(info: web::Data<VersionInfo>) -> HttpResponse {
    HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-store"))
        .json(info.get_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, test as actix_test};

    const SHA: &str = "2222222222222222222222222222222222222222";

    #[test]
    fn missing_settings_produce_null_fields() {
        for missing in [None, Some("  ".to_string())] {
            let info = VersionInfo::new(
                missing.clone(),
                Some("https://github.com/example/fork".into()),
            );
            assert_eq!(
                serde_json::to_value(info).unwrap(),
                serde_json::json!({"commit": null, "commit_url": null})
            );
            let info = VersionInfo::new(Some(SHA.into()), missing);
            assert_eq!(
                serde_json::to_value(info).unwrap(),
                serde_json::json!({"commit": SHA, "commit_url": null})
            );
        }
    }

    #[actix_web::test]
    async fn version_returns_commit_and_configured_url_without_caching() {
        let info = VersionInfo::new(
            Some(SHA.into()),
            Some("https://github.com/example/fork/".into()),
        );
        let app = actix_test::init_service(
            App::new()
                .app_data(web::Data::new(info))
                .route("/version", web::get().to(get_version)),
        )
        .await;
        let response = actix_test::call_service(
            &app,
            actix_test::TestRequest::get().uri("/version").to_request(),
        )
        .await;
        assert!(response.status().is_success());
        assert_eq!(response.headers().get("cache-control").unwrap(), "no-store");
        let body: serde_json::Value = actix_test::read_body_json(response).await;
        assert_eq!(
            body,
            serde_json::json!({
                "commit": SHA,
                "commit_url": "https://github.com/example/fork/commit/2222222222222222222222222222222222222222"
            })
        );
    }
}
