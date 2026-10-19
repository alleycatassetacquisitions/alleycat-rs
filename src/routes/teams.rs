use crate::events::{get_active_event_id, lock_active_event};
use crate::routes::errors;
use actix_web::{HttpResponse, error, get, patch, post, web};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TeamName {
    /// After trimming: 1–256 Unicode scalar values, without null characters.
    /// Unique within the event (case-sensitive).
    name: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct TeamResponse {
    id: Uuid,
    event_id: Uuid,
    name: String,
}

fn validate_name(name: &str) -> actix_web::Result<()> {
    let name = name.trim();
    // At most 1,024 UTF-8 bytes, safely below the unique index's entry limit.
    if name.is_empty() || name.chars().count() > 256 || name.contains('\0') {
        return Err(error::ErrorBadRequest(
            "Team name must contain 1–256 Unicode scalar values after trimming and no null characters",
        ));
    }
    Ok(())
}

fn database_error(error: sqlx::Error) -> actix_web::Error {
    if error.as_database_error().and_then(|e| e.constraint()) == Some("teams_event_name_key") {
        return error::ErrorConflict("Team name already exists in this event");
    }
    errors::database_error(error, "Team operation failed")
}

#[utoipa::path(
    tag = "Teams", summary = "Create a team in the active event",
    description = "Uses the active event; returns 409 when none is selected. Names follow the TeamName rules. Creates no player memberships.",
    request_body(content = TeamName, example = json!({"name": "Runners"})),
    responses(
        (status = 201, description = "Team created", body = TeamResponse),
        (status = 400, description = "Invalid JSON, unknown fields, or invalid name"),
        (status = 409, description = "No active event or name already exists in this event"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain")
    )
)]
#[post("/teams")]
pub async fn create_team(
    body: web::Json<TeamName>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    validate_name(&body.name)?;
    let mut transaction = pool.begin().await.map_err(database_error)?;
    let event_id = lock_active_event(&mut transaction)
        .await
        .map_err(database_error)?
        .ok_or_else(|| error::ErrorConflict("No active event"))?;
    let team = sqlx::query_as!(
        TeamResponse,
        "INSERT INTO teams (event_id, name)
         SELECT id, $2 FROM events WHERE id = $1
         RETURNING id, event_id, name",
        event_id,
        body.name.trim()
    )
    .fetch_one(&mut *transaction)
    .await
    .map_err(database_error)?;
    transaction.commit().await.map_err(database_error)?;
    Ok(HttpResponse::Created().json(team))
}

#[utoipa::path(
    tag = "Teams", summary = "Rename a team",
    description = "Requires name, following the TeamName rules. The team must belong to the active event; returns 409 when no event is selected. Preserves its ID and memberships; renaming to its current name succeeds.",
    params(
        ("id" = Uuid, Path, description = "Team ID")
    ),
    request_body(content = TeamName, example = json!({"name": "Runners"})),
    responses(
        (status = 200, description = "Renamed team", body = TeamResponse),
        (status = 400, description = "Invalid JSON, unknown fields, or invalid name"),
        (status = 404, description = "Team not found in the active event or invalid ID"),
        (status = 409, description = "No active event or name already exists in this event"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain")
    )
)]
#[patch("/teams/{id}")]
pub async fn rename_team(
    path: web::Path<Uuid>,
    body: web::Json<TeamName>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    validate_name(&body.name)?;
    let id = *path;
    let mut transaction = pool.begin().await.map_err(database_error)?;
    let event_id = lock_active_event(&mut transaction)
        .await
        .map_err(database_error)?
        .ok_or_else(|| error::ErrorConflict("No active event"))?;
    let team = sqlx::query_as!(
        TeamResponse,
        "UPDATE teams SET name = $3 WHERE event_id = $1 AND id = $2
         RETURNING id, event_id, name",
        event_id,
        id,
        body.name.trim()
    )
    .fetch_optional(&mut *transaction)
    .await
    .map_err(database_error)?
    .ok_or_else(|| error::ErrorNotFound("Team not found in this event"))?;
    transaction.commit().await.map_err(database_error)?;
    Ok(HttpResponse::Ok().json(team))
}

#[utoipa::path(
    tag = "Teams", summary = "List teams in the active event",
    description = "Returns all teams ordered by name, then ID. Uses the active event; returns 409 when none is selected. New events have no teams.",
    responses(
        (status = 200, description = "Event teams", body = Vec<TeamResponse>),
        (status = 409, description = "No active event"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain")
    )
)]
#[get("/teams")]
pub async fn list_teams(pool: web::Data<PgPool>) -> actix_web::Result<HttpResponse> {
    let event_id = get_active_event_id(pool.get_ref())
        .await
        .map_err(database_error)?
        .ok_or_else(|| error::ErrorConflict("No active event"))?;
    let teams = sqlx::query_as!(
        TeamResponse,
        "SELECT id, event_id, name FROM teams WHERE event_id = $1 ORDER BY name, id",
        event_id
    )
    .fetch_all(pool.get_ref())
    .await
    .map_err(database_error)?;
    Ok(HttpResponse::Ok().json(teams))
}
