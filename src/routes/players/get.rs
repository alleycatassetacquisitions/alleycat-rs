use crate::events::get_active_event_id;
use crate::routes::errors::database_error;
use crate::routes::pagination::{Pagination, PaginationQuery};
use actix_web::get;
use actix_web::web;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Serialize, utoipa::ToSchema)]
struct PlayerResponse {
    id: Uuid,
    pdn_code: String,
    name: String,
    created_at: DateTime<Utc>,
    team_id: Option<Uuid>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct GetPlayersResponse {
    players: Vec<PlayerResponse>,
    pagination: Pagination,
}

#[utoipa::path(
    tag = "Players", summary = "List players in the active event",
    description = "Newest first. Returns an empty list when no event is active.",
    params(PaginationQuery),
    responses((status = 200, description = "Players and effective pagination", body = GetPlayersResponse),
        (status = 400, description = "Invalid pagination query"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain"))
)]
#[get("/players")]
#[tracing::instrument(name = "Retrieve players", skip(parameters))]
pub async fn get_players(
    parameters: web::Query<PaginationQuery>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<web::Json<GetPlayersResponse>> {
    let pagination = parameters.normalize();

    let players = fetch_players(&pool, &pagination)
        .await
        .map_err(|error| database_error(error, "Failed to retrieve players"))?;

    Ok(web::Json(GetPlayersResponse {
        players,
        pagination,
    }))
}

#[tracing::instrument(name = "Fetch players")]
async fn fetch_players(
    pool: &PgPool,
    pagination: &Pagination,
) -> Result<Vec<PlayerResponse>, sqlx::Error> {
    let Some(event_id) = get_active_event_id(pool).await? else {
        return Ok(Vec::new());
    };
    let (limit, offset) = pagination.limit_offset();
    sqlx::query_as!(
        PlayerResponse,
        r#"
        SELECT
            id, pdn_code, name, created_at, team_id
        FROM players
        WHERE event_id = $3
        ORDER BY created_at DESC, id DESC
        LIMIT $1 OFFSET $2
        "#,
        limit,
        offset,
        event_id
    )
    .fetch_all(pool)
    .await
}
