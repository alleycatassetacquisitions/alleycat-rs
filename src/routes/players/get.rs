use actix_web::{HttpResponse, web};

#[derive(serde::Deserialize)]
pub struct Parameters {
    page: Option<u32>,
    per_page: Option<u32>,
}

#[tracing::instrument(name = "Retrieve players", skip(_parameters))]
pub async fn get_players(_parameters: web::Query<Parameters>) -> HttpResponse {
    HttpResponse::Ok().finish()
}

#[tracing::instrument(name = "Get all players", skip(_parameters))]
pub async fn get_all_players(
    pool: &PgPool,
    page: Option<u32>,
    per_page: Option<u32>,
) -> Result<Vec<Player>, sqlx::Error> {
    let result = sqlx::query!(
        "SELECT * FROM players LIMIT $1 OFFSET $2",
        per_page.unwrap_or(10),
        page.unwrap_or(0) * per_page.unwrap_or(10)
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();
}
