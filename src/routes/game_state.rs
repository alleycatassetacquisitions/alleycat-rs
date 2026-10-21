use crate::events::lock_active_event;
use crate::routes::helpers::errors::database_error;
use crate::routes::scoring::ScoringState;
use actix_web::{HttpResponse, error, get, put, web};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Serialize, utoipa::ToSchema)]
struct ActiveEvent {
    id: Uuid,
    name: String,
    venue_name: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Serialize, utoipa::ToSchema)]
struct GameState {
    /// Null when no event is selected.
    active_event: Option<ActiveEvent>,
    /// Null when no event is selected; started_at is null while scoring is off.
    scoring: Option<ScoringState>,
    /// Null when no event is selected. Independent of the current scoring switch.
    accepting_delayed_scores: Option<bool>,
    /// Database clock in UTC, sampled in the same statement as game status.
    server_time_utc: DateTime<Utc>,
}

#[utoipa::path(
    tag = "Game state", summary = "Read current game status",
    description = "A single database snapshot of the app_state active event, its current scoring period, its delayed-score switch, and UTC server time. With no active event, active_event, scoring, and accepting_delayed_scores are null; time is always present. Use PUT /events/{id}/active to select an event and PUT /scoring and PUT /delayed-scores to change its settings.",
    responses((status = 200, description = "Current game status", body = GameState),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain"))
)]
#[get("/game-state")]
pub async fn get_game_state(pool: web::Data<PgPool>) -> actix_web::Result<HttpResponse> {
    // The singleton anchor returns a row even without app_state. The database
    // clock cannot be null; all event columns are nullable because of the joins.
    let row = sqlx::query!(
        r#"SELECT e.id AS "id?", e.name AS "name?", e.venue_name,
                  e.created_at AS "created_at?",
                  e.accepting_delayed_scores AS "accepting_delayed_scores?",
                  p.started_at AS "started_at?", clock_timestamp() AS "server_time_utc!"
           FROM (SELECT 1) anchor
           LEFT JOIN app_state a ON a.id = 1
           LEFT JOIN events e ON e.id = a.active_event_id
           LEFT JOIN event_scoring_periods p ON p.event_id = e.id AND p.closes_at IS NULL"#
    )
    .fetch_one(pool.get_ref())
    .await
    .map_err(game_state_error)?;
    let active_event = match (row.id, row.name, row.created_at) {
        (Some(id), Some(name), Some(created_at)) => Some(ActiveEvent {
            id,
            name,
            venue_name: row.venue_name,
            created_at,
        }),
        _ => None,
    };
    let scoring = active_event.as_ref().map(|_| ScoringState {
        enabled: row.started_at.is_some(),
        started_at: row.started_at,
    });
    Ok(HttpResponse::Ok().json(GameState {
        active_event,
        scoring,
        accepting_delayed_scores: row.accepting_delayed_scores,
        server_time_utc: row.server_time_utc,
    }))
}

#[derive(Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DelayedScores {
    enabled: bool,
}

#[utoipa::path(
    tag = "Game state", summary = "Enable or disable delayed scoring for the active event",
    description = "Reversible switch for the active event, initially enabled and independent of scoring on/off. Returns 409 when no event is selected. When disabled, newly received matches from closed scoring periods are saved as delayed_scores_disabled. Matches in the currently open period score normally. Matches outside all periods remain scoring_disabled. Already accepted results never change, including on retries after re-enabling.",
    request_body(content = DelayedScores, example = json!({"enabled": false})),
    responses((status = 200, description = "Resulting delayed-score setting", body = DelayedScores),
        (status = 400, description = "Invalid JSON, missing enabled, or unknown fields"),
        (status = 409, description = "No active event"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain"))
)]
#[put("/delayed-scores")]
pub async fn set_delayed_scores(
    body: web::Json<DelayedScores>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    let mut transaction = pool.begin().await.map_err(game_state_error)?;
    let event_id = lock_active_event(&mut transaction)
        .await
        .map_err(game_state_error)?
        .ok_or_else(|| error::ErrorConflict("No active event"))?;
    // UPDATE takes the same event row lock as scoring changes and ingestion.
    let enabled = sqlx::query_scalar!(
        "UPDATE events SET accepting_delayed_scores = $2 WHERE id = $1 RETURNING accepting_delayed_scores",
        event_id, body.enabled
    )
    .fetch_one(&mut *transaction)
    .await
    .map_err(game_state_error)?;
    transaction.commit().await.map_err(game_state_error)?;
    Ok(HttpResponse::Ok().json(DelayedScores { enabled }))
}

fn game_state_error(error: sqlx::Error) -> actix_web::Error {
    database_error(error, "Game state operation failed")
}
