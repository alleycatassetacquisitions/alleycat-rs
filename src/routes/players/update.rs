use super::PlayerResponse;
use crate::domain::{PlayerEmail, PlayerName};
use crate::events::lock_active_event;
use crate::routes::errors::database_error;
use actix_web::{HttpResponse, error, patch, web};
use serde::{Deserialize, Deserializer};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdatePlayer {
    /// Same validation as registration. Omission preserves the name.
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false)]
    name: Option<String>,
    /// Omission preserves the email; null clears it.
    #[serde(default, deserialize_with = "present")]
    email: Option<Option<String>>,
    /// Must be a team in this event. Null removes membership.
    #[serde(default, deserialize_with = "present")]
    team_id: Option<Option<Uuid>>,
    /// Reversible registration/device status; never changes match history.
    #[serde(default, deserialize_with = "present")]
    #[schema(nullable = false)]
    active: Option<bool>,
}

fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

fn update_error(error: sqlx::Error) -> actix_web::Error {
    match error
        .as_database_error()
        .and_then(|error| error.constraint())
    {
        Some("players_event_name_key") => {
            error::ErrorConflict("Player name already exists in the active event")
        }
        Some("players_event_team_fkey") => {
            error::ErrorBadRequest("Team must belong to the player's event")
        }
        _ => database_error(error, "Failed to update player"),
    }
}

#[utoipa::path(
    tag = "Players", summary = "Update a player in the active event",
    description = "Supply at least one of name, email, team_id, or active. Omitted fields remain unchanged; email and team_id may be null to clear them. Name uses registration validation. Teams must belong to this event; an existing membership may be replaced. UUID, event, PIN, and creation time are immutable. Inactive registrations remain listed and discoverable and keep their names/PINs. Team/status changes never rewrite scores or prevent delayed uploads. Unknown fields are rejected.",
    params(("id" = Uuid, Path, description = "Player ID")),
    request_body(content = UpdatePlayer, example = json!({"name": "Alex Updated", "email": null, "active": true})),
    responses((status = 200, description = "Full updated player details", body = PlayerResponse),
        (status = 400, description = "Invalid JSON, fields, name/email, or team; no fields supplied"),
        (status = 404, description = "Player not found in active event or invalid UUID"),
        (status = 409, description = "No active event or duplicate name in active event", body = String, content_type = "text/plain"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain"))
)]
#[patch("/players/{id}")]
pub async fn update_player(
    id: web::Path<Uuid>,
    body: web::Json<UpdatePlayer>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    if body.name.is_none()
        && body.email.is_none()
        && body.team_id.is_none()
        && body.active.is_none()
    {
        return Err(error::ErrorBadRequest(
            "Supply name, email, team_id, or active",
        ));
    }
    let name = body
        .name
        .clone()
        .map(PlayerName::parse)
        .transpose()
        .map_err(error::ErrorBadRequest)?;
    let email = body
        .email
        .clone()
        .flatten()
        .map(PlayerEmail::parse)
        .transpose()
        .map_err(error::ErrorBadRequest)?;
    let mut transaction = pool.begin().await.map_err(update_error)?;
    let event_id = lock_active_event(&mut transaction)
        .await
        .map_err(update_error)?
        .ok_or_else(|| error::ErrorConflict("No active event"))?;
    // One UPDATE preserves omitted fields even when concurrent requests change
    // different fields. Keep the active selection locked through commit.
    let player = sqlx::query_as!(
        PlayerResponse,
        r#"WITH updated AS (
            UPDATE players SET name = COALESCE($3, name),
                email = CASE WHEN $4 THEN $5 ELSE email END,
                team_id = CASE WHEN $6 THEN $7 ELSE team_id END,
                active = COALESCE($8, active)
            WHERE id = $1 AND event_id = $2 RETURNING *
        )
        SELECT p.id, p.event_id, p.pdn_code, p.name, p.email, p.created_at, p.team_id,
               t.name AS "team_name?", p.active
        FROM updated p LEFT JOIN teams t ON t.id = p.team_id AND t.event_id = p.event_id"#,
        *id,
        event_id,
        name.as_ref().map(|name| name.as_ref()),
        body.email.is_some(),
        email.as_ref().map(|email| email.as_ref()),
        body.team_id.is_some(),
        body.team_id.flatten(),
        body.active
    )
    .fetch_optional(&mut *transaction)
    .await
    .map_err(update_error)?
    .ok_or_else(|| error::ErrorNotFound("Player not found in the active event"))?;
    transaction.commit().await.map_err(update_error)?;
    Ok(HttpResponse::Ok().json(player))
}
