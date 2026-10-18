use crate::events::{get_active_event_id, lock_active_event};
use crate::routes::errors::database_error;
use actix_web::{HttpResponse, error, get, put, web};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SetScoring {
    enabled: bool,
}

#[derive(Serialize, utoipa::ToSchema)]
pub(super) struct ScoringState {
    pub(super) enabled: bool,
    /// Inclusive start of the current period; null while scoring is disabled.
    pub(super) started_at: Option<DateTime<Utc>>,
}

fn scoring_error(error: sqlx::Error) -> actix_web::Error {
    database_error(error, "Scoring operation failed")
}

#[utoipa::path(
    tag = "Scoring", summary = "Read event scoring state",
    description = "Uses the active event; returns 409 when none is selected. New events have scoring disabled. This is the current state; delayed match eligibility uses historical scoring periods.",
    responses(
        (status = 200, description = "Current scoring state", body = ScoringState),
        (status = 409, description = "No active event"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain")
    )
)]
#[get("/scoring")]
pub async fn get_scoring(pool: web::Data<PgPool>) -> actix_web::Result<HttpResponse> {
    let event_id = get_active_event_id(pool.get_ref())
        .await
        .map_err(scoring_error)?
        .ok_or_else(|| error::ErrorConflict("No active event"))?;
    let state = sqlx::query_as!(
        ScoringState,
        r#"SELECT (p.id IS NOT NULL) AS "enabled!", p.started_at AS "started_at?"
           FROM events e LEFT JOIN event_scoring_periods p
             ON p.event_id = e.id AND p.closes_at IS NULL
           WHERE e.id = $1"#,
        event_id
    )
    .fetch_one(pool.get_ref())
    .await
    .map_err(scoring_error)?;
    Ok(HttpResponse::Ok().json(state))
}

#[utoipa::path(
    tag = "Scoring", summary = "Enable or disable event scoring",
    description = "Uses the active event; returns 409 when none is selected. Sets the desired state using the database clock after serializing changes for this event. Repeated requests for the current state do nothing. Enabling opens a period; disabling closes it and preserves history. Start is inclusive and close is exclusive. Does not change previously accepted match results.",
    request_body(content = SetScoring, example = json!({"enabled": true})),
    responses(
        (status = 200, description = "Resulting scoring state", body = ScoringState),
        (status = 400, description = "Invalid JSON, missing enabled, or unknown fields"),
        (status = 409, description = "No active event"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain")
    )
)]
#[put("/scoring")]
pub async fn set_scoring(
    body: web::Json<SetScoring>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    let mut transaction = pool.begin().await.map_err(scoring_error)?;
    let event_id = lock_active_event(&mut transaction)
        .await
        .map_err(scoring_error)?
        .ok_or_else(|| error::ErrorConflict("No active event"))?;
    // Lock the parent even when no period exists so concurrent first enables
    // serialize too. Read history in a separate statement after acquiring it.
    sqlx::query_scalar!("SELECT id FROM events WHERE id = $1 FOR UPDATE", event_id)
        .fetch_one(&mut *transaction)
        .await
        .map_err(scoring_error)?;

    let mut started_at = sqlx::query_scalar!(
        "SELECT started_at FROM event_scoring_periods WHERE event_id = $1 AND closes_at IS NULL",
        event_id
    )
    .fetch_optional(&mut *transaction)
    .await
    .map_err(scoring_error)?;

    if body.enabled && started_at.is_none() {
        // Keep periods ordered even if the database wall clock moves backward.
        started_at = Some(
            sqlx::query_scalar!(
                "INSERT INTO event_scoring_periods (event_id, started_at)
             SELECT $1, GREATEST(clock_timestamp(), MAX(closes_at))
             FROM event_scoring_periods WHERE event_id = $1
             RETURNING started_at",
                event_id
            )
            .fetch_one(&mut *transaction)
            .await
            .map_err(scoring_error)?,
        );
    } else if !body.enabled && started_at.is_some() {
        sqlx::query!(
            "UPDATE event_scoring_periods
             SET closes_at = GREATEST(clock_timestamp(), started_at + interval '1 microsecond')
             WHERE event_id = $1 AND closes_at IS NULL",
            event_id
        )
        .execute(&mut *transaction)
        .await
        .map_err(scoring_error)?;
        started_at = None;
    }
    transaction.commit().await.map_err(scoring_error)?;
    Ok(HttpResponse::Ok().json(ScoringState {
        enabled: started_at.is_some(),
        started_at,
    }))
}
