use crate::routes::errors::database_error;
use actix_web::{HttpResponse, error, get, put, web};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SetScoring {
    enabled: bool,
}

#[derive(Serialize, utoipa::ToSchema)]
struct ScoringState {
    enabled: bool,
    /// Inclusive start of the current period; null while scoring is disabled.
    started_at: Option<DateTime<Utc>>,
}

fn scoring_error(error: sqlx::Error) -> actix_web::Error {
    database_error(error, "Scoring operation failed")
}

#[utoipa::path(
    tag = "Scoring", summary = "Read event scoring state",
    description = "Uses the explicit event regardless of the active event. New events have scoring disabled. This is the current state; delayed match eligibility uses historical scoring periods.",
    params(("event_id" = Uuid, Path, description = "Event ID")),
    responses(
        (status = 200, description = "Current scoring state", body = ScoringState),
        (status = 404, description = "Event not found or invalid event ID"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain")
    )
)]
#[get("/events/{event_id}/scoring")]
pub async fn get_scoring(
    event_id: web::Path<Uuid>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    let state = sqlx::query_as!(
        ScoringState,
        r#"SELECT (p.id IS NOT NULL) AS "enabled!", p.started_at AS "started_at?"
           FROM events e LEFT JOIN event_scoring_periods p
             ON p.event_id = e.id AND p.closes_at IS NULL
           WHERE e.id = $1"#,
        *event_id
    )
    .fetch_optional(pool.get_ref())
    .await
    .map_err(scoring_error)?
    .ok_or_else(|| error::ErrorNotFound("Event not found"))?;
    Ok(HttpResponse::Ok().json(state))
}

#[utoipa::path(
    tag = "Scoring", summary = "Enable or disable event scoring",
    description = "Sets the desired state using the database clock after serializing changes for this event. Repeated requests for the current state do nothing. Enabling opens a period; disabling closes it and preserves history. Start is inclusive and close is exclusive. Does not change previously accepted match results.",
    params(("event_id" = Uuid, Path, description = "Event ID")),
    request_body(content = SetScoring, example = json!({"enabled": true})),
    responses(
        (status = 200, description = "Resulting scoring state", body = ScoringState),
        (status = 400, description = "Invalid JSON, missing enabled, or unknown fields"),
        (status = 404, description = "Event not found or invalid event ID"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain")
    )
)]
#[put("/events/{event_id}/scoring")]
pub async fn set_scoring(
    event_id: web::Path<Uuid>,
    body: web::Json<SetScoring>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    let mut transaction = pool.begin().await.map_err(scoring_error)?;
    // Lock the parent even when no period exists so concurrent first enables
    // serialize too. Read history in a separate statement after acquiring it.
    sqlx::query_scalar!("SELECT id FROM events WHERE id = $1 FOR UPDATE", *event_id)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(scoring_error)?
        .ok_or_else(|| error::ErrorNotFound("Event not found"))?;

    let mut started_at = sqlx::query_scalar!(
        "SELECT started_at FROM event_scoring_periods WHERE event_id = $1 AND closes_at IS NULL",
        *event_id
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
                *event_id
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
            *event_id
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
