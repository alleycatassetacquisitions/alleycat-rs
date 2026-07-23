use crate::domain::PlayerMode;
use actix_web::{error::ErrorInternalServerError, web};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct GetPlayersQuery {
    page: Option<u32>,
    per_page: Option<u32>,
}

#[derive(Serialize)]
struct PlayerResponse {
    id: Uuid,
    pdn_code: String,
    name: String,
    created_at: DateTime<Utc>,
    mode: PlayerMode,
}

#[derive(Serialize)]
struct Pagination {
    page: u32,
    per_page: u32,
}

#[derive(Serialize)]
pub struct GetPlayersResponse {
    players: Vec<PlayerResponse>,
    pagination: Pagination,
}

#[tracing::instrument(name = "Retrieve players", skip(parameters))]
pub async fn get_players(
    parameters: web::Query<GetPlayersQuery>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<web::Json<GetPlayersResponse>> {
    const DEFAULT_PAGE: u32 = 1;
    const DEFAULT_PER_PAGE: u32 = 20;
    const MAX_PER_PAGE: u32 = 100;

    let page = parameters.page.unwrap_or(DEFAULT_PAGE).max(1);
    let per_page = parameters
        .per_page
        .unwrap_or(DEFAULT_PER_PAGE)
        .clamp(1, MAX_PER_PAGE);

    let players = fetch_players(&pool, page, per_page)
        .await
        .map_err(|error| {
            tracing::error!(?error, "Failed to retrieve players");
            ErrorInternalServerError("Failed to retrieve players")
        })?;

    Ok(web::Json(GetPlayersResponse {
        players,
        pagination: Pagination { page, per_page },
    }))
}

#[tracing::instrument(name = "Fetch players")]
async fn fetch_players(
    pool: &PgPool,
    page: u32,
    per_page: u32,
) -> Result<Vec<PlayerResponse>, sqlx::Error> {
    let limit = i64::from(per_page);
    let offset = i64::from(page - 1) * limit;
    sqlx::query_as!(
        PlayerResponse,
        r#"
        SELECT
            id, pdn_code, name, created_at,
            mode AS "mode: PlayerMode"
        FROM players
        ORDER BY created_at DESC, id DESC
        LIMIT $1 OFFSET $2
        "#,
        limit,
        offset
    )
    .fetch_all(pool)
    .await
}
