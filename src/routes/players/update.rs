use super::PlayerResponse;
use crate::domain::{PlayerEmail, PlayerName};
use crate::events::lock_active_event;
use crate::routes::errors::database_error;
use actix_web::{HttpResponse, error, patch, web};
use serde::{Deserialize, Deserializer};
use sqlx::PgPool;
use std::collections::BTreeMap;
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
    /// Changes keyed by category UUID. Each value is an affiliation UUID or null to clear
    /// that category. Omitted categories stay unchanged; an empty object changes nothing.
    /// Both category and affiliation must belong to this player's event, and the affiliation
    /// must belong to that category. The object itself cannot be null.
    #[serde(default)]
    affiliations: BTreeMap<Uuid, Option<Uuid>>,
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
    description = "Supply name, email, team_id, active, or at least one entry in affiliations. Omitted fields remain unchanged; email and team_id may be null to clear them. Name uses registration validation. Teams must belong to this event; an existing membership may be replaced. UUID, event, PIN, and creation time are immutable. Inactive registrations remain listed and discoverable and keep their names/PINs. Team/status changes never rewrite scores or prevent delayed uploads. Unknown fields are rejected. Affiliations is an object keyed by category UUID: an affiliation UUID assigns or replaces that category, and null clears it. Omitted categories remain unchanged. All player and affiliation changes succeed or fail together. Returns full player details with current affiliation IDs and names. Affiliations do not change permissions or scores. Current assignments are also included in device protobuf lookups.",
    params(("id" = Uuid, Path, description = "Player ID")),
    request_body(content = UpdatePlayer, example = json!({"name": "Alex Updated", "affiliations": {"11111111-1111-4111-8111-111111111111": "22222222-2222-4222-8222-222222222222", "33333333-3333-4333-8333-333333333333": null}})),
    responses((status = 200, description = "Full updated player details", body = PlayerResponse),
        (status = 400, description = "Invalid JSON, fields, name/email, team, or affiliation/category; no changes supplied"),
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
        && body.affiliations.is_empty()
    {
        return Err(error::ErrorBadRequest(
            "Supply name, email, team_id, active, or at least one affiliation change",
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
    // different fields. Its row lock also serializes affiliation changes. Keep
    // both the player and active selection locked through commit.
    let updated = sqlx::query(
        "UPDATE players SET name = COALESCE($3, name),
            email = CASE WHEN $4 THEN $5 ELSE email END,
            team_id = CASE WHEN $6 THEN $7 ELSE team_id END,
            active = COALESCE($8, active)
         WHERE id = $1 AND event_id = $2",
    )
    .bind(*id)
    .bind(event_id)
    .bind(name.as_ref().map(|name| name.as_ref()))
    .bind(body.email.is_some())
    .bind(email.as_ref().map(|email| email.as_ref()))
    .bind(body.team_id.is_some())
    .bind(body.team_id.flatten())
    .bind(body.active)
    .execute(&mut *transaction)
    .await
    .map_err(update_error)?;

    if updated.rows_affected() == 0 {
        return Err(error::ErrorNotFound("Player not found in the active event"));
    }
    for (&category_id, &affiliation_id) in &body.affiliations {
        let category_exists = sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM affiliation_categories WHERE id = $1 AND event_id = $2) AS "exists!""#,
            category_id, event_id
        )
        .fetch_one(&mut *transaction)
        .await
        .map_err(update_error)?;
        if !category_exists {
            return Err(error::ErrorBadRequest(
                "Category must belong to the player's event",
            ));
        }
        if let Some(affiliation_id) = affiliation_id {
            sqlx::query_scalar!(
                "INSERT INTO player_affiliations (event_id, player_id, category_id, affiliation_id)
                 SELECT event_id, $1, category_id, id FROM affiliations
                 WHERE id = $2 AND event_id = $3 AND category_id = $4
                 ON CONFLICT (player_id, category_id) DO UPDATE SET affiliation_id = EXCLUDED.affiliation_id
                 RETURNING affiliation_id",
                *id, affiliation_id, event_id, category_id
            )
            .fetch_optional(&mut *transaction)
            .await
            .map_err(update_error)?
            .ok_or_else(|| {
                error::ErrorBadRequest("Affiliation must belong to this category and event")
            })?;
        } else {
            sqlx::query!(
                "DELETE FROM player_affiliations WHERE event_id = $1 AND player_id = $2 AND category_id = $3",
                event_id, *id, category_id
            )
            .execute(&mut *transaction)
            .await
            .map_err(update_error)?;
        }
    }
    // Read after all writes so the response includes the newly saved assignments.
    let player = sqlx::query_as::<_, PlayerResponse>(
        r#"SELECT p.id, p.event_id, p.pdn_code, p.name, p.email, p.created_at, p.team_id,
                  t.name AS team_name, p.active,
            COALESCE((
                SELECT jsonb_agg(jsonb_build_object(
                    'category_id', c.id, 'category_name', c.name,
                    'affiliation_id', a.id, 'affiliation_name', a.name
                ) ORDER BY c.name, c.id)
                FROM player_affiliations pa
                JOIN affiliation_categories c ON c.id = pa.category_id AND c.event_id = pa.event_id
                JOIN affiliations a ON a.id = pa.affiliation_id AND a.category_id = pa.category_id AND a.event_id = pa.event_id
                WHERE pa.player_id = p.id AND pa.event_id = p.event_id
            ), '[]'::jsonb) AS affiliations
           FROM players p LEFT JOIN teams t ON t.id = p.team_id AND t.event_id = p.event_id
           WHERE p.id = $1 AND p.event_id = $2"#,
    )
    .bind(*id)
    .bind(event_id)
    .fetch_one(&mut *transaction)
    .await
    .map_err(update_error)?;
    transaction.commit().await.map_err(update_error)?;
    Ok(HttpResponse::Ok().json(player))
}
