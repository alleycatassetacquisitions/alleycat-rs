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

#[derive(Serialize, sqlx::FromRow, utoipa::ToSchema)]
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
    tracing::error!(?error, "Team operation failed");
    error::ErrorInternalServerError("Team operation failed")
}

#[utoipa::path(
    tag = "Teams", summary = "Create a team in an event",
    description = "Uses the explicit event, not the active event. Names follow the TeamName rules. Creates no player memberships.",
    params(("event_id" = Uuid, Path, description = "Event ID")),
    request_body(content = TeamName, example = json!({"name": "Runners"})),
    responses(
        (status = 201, description = "Team created", body = TeamResponse),
        (status = 400, description = "Invalid JSON, unknown fields, or invalid name"),
        (status = 404, description = "Event not found or invalid event ID"),
        (status = 409, description = "Name already exists in this event"),
        (status = 500, description = "Database operation failed")
    )
)]
#[post("/events/{event_id}/teams")]
pub async fn create_team(
    event_id: web::Path<Uuid>,
    body: web::Json<TeamName>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    validate_name(&body.name)?;
    let team = sqlx::query_as::<_, TeamResponse>(
        "INSERT INTO teams (event_id, name)
         SELECT id, $2 FROM events WHERE id = $1
         RETURNING id, event_id, name",
    )
    .bind(*event_id)
    .bind(body.name.trim())
    .fetch_optional(pool.get_ref())
    .await
    .map_err(database_error)?
    .ok_or_else(|| error::ErrorNotFound("Event not found"))?;
    Ok(HttpResponse::Created().json(team))
}

#[utoipa::path(
    tag = "Teams", summary = "Rename a team",
    description = "Requires name, following the TeamName rules. The team must belong to the specified event. Preserves its ID and memberships; renaming to its current name succeeds.",
    params(
        ("event_id" = Uuid, Path, description = "Event ID"),
        ("id" = Uuid, Path, description = "Team ID")
    ),
    request_body(content = TeamName, example = json!({"name": "Runners"})),
    responses(
        (status = 200, description = "Renamed team", body = TeamResponse),
        (status = 400, description = "Invalid JSON, unknown fields, or invalid name"),
        (status = 404, description = "Team not found in this event or invalid ID"),
        (status = 409, description = "Name already exists in this event"),
        (status = 500, description = "Database operation failed")
    )
)]
#[patch("/events/{event_id}/teams/{id}")]
pub async fn rename_team(
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<TeamName>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    validate_name(&body.name)?;
    let (event_id, id) = path.into_inner();
    let team = sqlx::query_as::<_, TeamResponse>(
        "UPDATE teams SET name = $3 WHERE event_id = $1 AND id = $2
         RETURNING id, event_id, name",
    )
    .bind(event_id)
    .bind(id)
    .bind(body.name.trim())
    .fetch_optional(pool.get_ref())
    .await
    .map_err(database_error)?
    .ok_or_else(|| error::ErrorNotFound("Team not found in this event"))?;
    Ok(HttpResponse::Ok().json(team))
}

#[utoipa::path(
    tag = "Teams", summary = "List teams in an event",
    description = "Returns all teams ordered by name, then ID. Uses the explicit event regardless of the active selection. New events have no teams.",
    params(("event_id" = Uuid, Path, description = "Event ID")),
    responses(
        (status = 200, description = "Event teams", body = Vec<TeamResponse>),
        (status = 404, description = "Event not found or invalid event ID"),
        (status = 500, description = "Database operation failed")
    )
)]
#[get("/events/{event_id}/teams")]
pub async fn list_teams(
    event_id: web::Path<Uuid>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM events WHERE id = $1)")
        .bind(*event_id)
        .fetch_one(pool.get_ref())
        .await
        .map_err(database_error)?;
    if !exists {
        return Err(error::ErrorNotFound("Event not found"));
    }
    let teams = sqlx::query_as::<_, TeamResponse>(
        "SELECT id, event_id, name FROM teams WHERE event_id = $1 ORDER BY name, id",
    )
    .bind(*event_id)
    .fetch_all(pool.get_ref())
    .await
    .map_err(database_error)?;
    Ok(HttpResponse::Ok().json(teams))
}
