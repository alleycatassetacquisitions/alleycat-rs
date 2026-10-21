use super::{PlayerAffiliation, PlayerResponse};
use crate::proto::alleycat::device::{DevicePlayerAffiliation, DevicePlayerResponse};
use crate::routes::helpers::device_display::display_text;
use crate::routes::helpers::errors::database_error;
use crate::routes::helpers::pagination::{Pagination, PaginationQuery};
use crate::routes::helpers::protobuf::ProtobufBody;
use actix_web::get;
use actix_web::{HttpResponse, error, web};
use prost::Message;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Serialize, utoipa::ToSchema)]
pub struct GetPlayersResponse {
    players: Vec<PlayerResponse>,
    pagination: Pagination,
}

#[utoipa::path(
    tag = "Players", summary = "List players in the active event",
    description = "Newest first, including inactive registrations. Returns full player details with email, current team name, and category/affiliation IDs and names. Returns an empty list when no event is active.",
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
    let (limit, offset) = pagination.limit_offset();
    sqlx::query_as::<_, PlayerResponse>(
        r#"
        SELECT
            p.id, p.event_id, p.pdn_code, p.name, p.email, p.created_at, p.team_id,
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
        WHERE p.event_id = (SELECT active_event_id FROM app_state WHERE id = 1)
        ORDER BY p.created_at DESC, p.id DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

fn validate_pin(pin: &str) -> actix_web::Result<()> {
    if pin.len() != 4 || !pin.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(error::ErrorBadRequest(
            "PIN must be exactly four ASCII digits",
        ));
    }
    Ok(())
}

async fn fetch_player(
    pool: &PgPool,
    id: Option<Uuid>,
    pin: Option<&str>,
) -> actix_web::Result<PlayerResponse> {
    sqlx::query_as::<_, PlayerResponse>(
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
           WHERE p.event_id = (SELECT active_event_id FROM app_state WHERE id = 1)
             AND (p.id = $1 OR p.pdn_code = $2)"#,
    )
    .bind(id)
    .bind(pin)
    .fetch_optional(pool)
    .await
    .map_err(|error| database_error(error, "Failed to retrieve player"))?
    .ok_or_else(|| error::ErrorNotFound("Player not found in the active event"))
}

#[utoipa::path(
    tag = "Players", summary = "Get a player by UUID",
    description = "Returns full player details in the active event, including inactive registrations and current affiliation assignments. Returns 404 when no event is active or the player belongs to another event.",
    params(("id" = Uuid, Path, description = "Player ID")),
    responses((status = 200, description = "Player details", body = PlayerResponse),
        (status = 404, description = "Player not found in the active event or invalid UUID"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain"))
)]
#[get("/players/{id}")]
pub async fn get_player(
    id: web::Path<Uuid>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(fetch_player(&pool, Some(*id), None).await?))
}

#[utoipa::path(
    tag = "Players", summary = "Get a player by PIN",
    description = "Looks up the four-digit pdn_code in the active event. Preserve leading zeros. Includes email, team details, active/inactive status, and current affiliation assignments. PINs may repeat across events, so this always uses the active selection.",
    params(("pin" = String, Path, description = "Exactly four ASCII digits, e.g. 0001")),
    responses((status = 200, description = "Player details", body = PlayerResponse),
        (status = 400, description = "Invalid PIN format"),
        (status = 404, description = "Player not found in the active event or no active event"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain"))
)]
#[get("/players/by-pin/{pin}")]
pub async fn get_player_by_pin(
    pin: web::Path<String>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    validate_pin(&pin)?;
    Ok(HttpResponse::Ok().json(fetch_player(&pool, None, Some(&pin)).await?))
}

#[utoipa::path(
    tag = "Players", summary = "Get device player details by PIN (protobuf)",
    description = "Returns binary alleycat.device.DevicePlayerResponse from the active event, including inactive registrations. Only affiliations in categories enabled for devices are included. Preserve leading zeros in the path PIN. GET has no request body. See /docs/device-apis#alleycat.device.DevicePlayerResponse for fields, UUID encoding, and PDN display constraints.",
    params(("pin" = String, Path, description = "Exactly four ASCII digits, e.g. 0001")),
    responses((status = 200, description = "Device player details", body = inline(ProtobufBody), content_type = "application/protobuf"),
        (status = 400, description = "Invalid PIN format", body = String, content_type = "text/plain"),
        (status = 404, description = "Player not found in the active event or no active event", body = String, content_type = "text/plain"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain"))
)]
#[get("/device/players/by-pin/{pin}")]
pub async fn get_device_player_by_pin(
    pin: web::Path<String>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    validate_pin(&pin)?;
    // Select only device-visible fields; staff details are never loaded on this path.
    let row = sqlx::query!(
        r#"SELECT p.id, p.name, p.team_id, t.name AS "team_name?",
            COALESCE((
                SELECT jsonb_agg(jsonb_build_object(
                    'category_id', c.id, 'category_name', c.name,
                    'affiliation_id', a.id, 'affiliation_name', a.name
                ) ORDER BY c.name, c.id)
                FROM player_affiliations pa
                JOIN affiliation_categories c ON c.id = pa.category_id AND c.event_id = pa.event_id
                JOIN affiliations a ON a.id = pa.affiliation_id AND a.category_id = pa.category_id AND a.event_id = pa.event_id
                WHERE pa.player_id = p.id AND pa.event_id = p.event_id
                  AND c.send_to_devices
            ), '[]'::jsonb) AS "affiliations!: sqlx::types::Json<Vec<PlayerAffiliation>>"
           FROM players p LEFT JOIN teams t ON t.id = p.team_id AND t.event_id = p.event_id
           WHERE p.event_id = (SELECT active_event_id FROM app_state WHERE id = 1)
             AND p.pdn_code = $1"#,
        pin.as_str()
    )
    .fetch_optional(pool.get_ref())
    .await
    .map_err(|error| database_error(error, "Failed to retrieve player"))?
    .ok_or_else(|| error::ErrorNotFound("Player not found in the active event"))?;
    let message = DevicePlayerResponse {
        id: row.id.as_bytes().to_vec(),
        name: display_text(&row.name),
        team_id: row.team_id.map(|id| id.as_bytes().to_vec()),
        team_name: row.team_name.as_deref().map(display_text),
        affiliations: row
            .affiliations
            .0
            .into_iter()
            .map(|assignment| DevicePlayerAffiliation {
                category_id: assignment.category_id.as_bytes().to_vec(),
                category_name: display_text(&assignment.category_name),
                affiliation_id: assignment.affiliation_id.as_bytes().to_vec(),
                affiliation_name: display_text(&assignment.affiliation_name),
            })
            .collect(),
    };
    Ok(HttpResponse::Ok()
        .content_type("application/protobuf")
        .body(message.encode_to_vec()))
}

#[cfg(test)]
mod tests {
    use super::validate_pin;

    #[test]
    fn pins_preserve_four_ascii_digits_including_leading_zeros() {
        for pin in ["0001", "1234", "9999"] {
            assert!(validate_pin(pin).is_ok());
        }
        for pin in ["", "1", "12345", "abcd", " 123", "１２３４", "١٢٣٤"] {
            assert!(validate_pin(pin).is_err());
        }
    }
}
