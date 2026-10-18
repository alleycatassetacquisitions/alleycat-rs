use super::PlayerResponse;
use crate::domain::{NewPlayer, PlayerEmail, PlayerName};
use crate::events::lock_active_event;
use crate::routes::errors::database_error;
use actix_web::post;
use actix_web::{HttpResponse, web};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct RegisterPlayer {
    /// Nonblank name, at most 256 graphemes; cannot contain null characters or / ( ) \" < > \\ { }.
    #[schema(example = "Alex")]
    name: String,
    /// Optional valid email address.
    #[schema(example = "alex@example.com")]
    email: Option<String>,
}

impl TryFrom<RegisterPlayer> for NewPlayer {
    type Error = String;

    fn try_from(value: RegisterPlayer) -> Result<Self, Self::Error> {
        let name = PlayerName::parse(value.name)?;
        let email = value.email.map(PlayerEmail::parse).transpose()?;
        Ok(NewPlayer { email, name })
    }
}

#[utoipa::path(
    tag = "Players", summary = "Register a player in the active event",
    description = "Creates a player in the active event and allocates a four-digit PDN sequentially, skipping reserved and already assigned codes. New players have no team (team_id is null) and no affiliations (affiliations is an empty array). Send application/json with name and optional email. Email may be omitted or null.",
    request_body(content = RegisterPlayer, content_type = "application/json", example = json!({"name": "Alex", "email": "alex@example.com"})),
    responses((status = 200, description = "Registered player with assigned PIN, email, team details, and status", body = PlayerResponse),
        (status = 400, description = "Invalid JSON or Content-Type, or missing or invalid name or email"),
        (status = 409, description = "No active event, duplicate name in the active event, or no PDN codes remaining. Text body without a Content-Type header.", body = String, content_type = "text/plain"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain"))
)]
#[post("/players")]
#[tracing::instrument(
    name = "Registering player",
    skip(body, pool),
    fields(
        name = %body.name,
        email = %body.email.is_some(),
    )
)]
pub async fn register_player(
    body: web::Json<RegisterPlayer>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    let new_player = match body.0.try_into() {
        Ok(player) => player,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    match insert_player(&pool, &new_player).await {
        Ok(player) => HttpResponse::Ok().json(player),
        Err(RegistrationError::NoActiveEvent) => HttpResponse::Conflict().body("No active event"),
        Err(RegistrationError::CodesExhausted) => {
            HttpResponse::Conflict().body("No PDN codes available for the active event")
        }
        Err(RegistrationError::DuplicateName) => {
            HttpResponse::Conflict().body("Player name already exists in the active event")
        }
        Err(RegistrationError::Database(error)) => {
            database_error(error, "Failed to register player").error_response()
        }
    }
}

#[derive(Debug)]
enum RegistrationError {
    NoActiveEvent,
    CodesExhausted,
    DuplicateName,
    Database(sqlx::Error),
}

impl From<sqlx::Error> for RegistrationError {
    fn from(error: sqlx::Error) -> Self {
        if error.as_database_error().and_then(|e| e.constraint()) == Some("players_event_name_key")
        {
            Self::DuplicateName
        } else {
            Self::Database(error)
        }
    }
}

#[tracing::instrument(name = "Adding a player to the database", skip(new_player, pool))]
async fn insert_player(
    pool: &PgPool,
    new_player: &NewPlayer,
) -> Result<PlayerResponse, RegistrationError> {
    let mut transaction = pool.begin().await?;
    // Keep the selection locked through allocation, persistence, and commit.
    let event_id = lock_active_event(&mut transaction)
        .await?
        .ok_or(RegistrationError::NoActiveEvent)?;
    // Serialize allocations within each event, including concurrent requests.
    let next_code = sqlx::query_scalar!(
        "SELECT next_pdn_code FROM events WHERE id = $1 FOR UPDATE",
        event_id
    )
    .fetch_one(&mut *transaction)
    .await?;
    let code = sqlx::query_scalar!(
        r#"
        SELECT lpad(n::text, 4, '0') AS "code!"
        FROM generate_series($1::integer, 9999) AS n
        WHERE NOT EXISTS (
            SELECT 1 FROM reserved_pdn_codes WHERE code = lpad(n::text, 4, '0')
        ) AND NOT EXISTS (
            SELECT 1 FROM players WHERE event_id = $2 AND pdn_code = lpad(n::text, 4, '0')
        )
        ORDER BY n
        LIMIT 1
        "#,
        next_code,
        event_id
    )
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or(RegistrationError::CodesExhausted)?;

    let player = sqlx::query_as::<_, PlayerResponse>(
        r#"WITH inserted AS (
            INSERT INTO players (id, event_id, pdn_code, name, email, created_at)
            VALUES ($1, $2, $3, $4, $5, $6) RETURNING *
        )
        SELECT p.id, p.event_id, p.pdn_code, p.name, p.email, p.created_at, p.team_id,
               t.name AS team_name, p.active, '[]'::jsonb AS affiliations
        FROM inserted p LEFT JOIN teams t ON t.id = p.team_id AND t.event_id = p.event_id"#,
    )
    .bind(Uuid::new_v4())
    .bind(event_id)
    .bind(&code)
    .bind(new_player.name.as_ref())
    .bind(new_player.email.as_ref().map(|email| email.as_ref()))
    .bind(Utc::now())
    .fetch_one(&mut *transaction)
    .await?;
    sqlx::query!(
        "UPDATE events SET next_pdn_code = $2::text::integer + 1 WHERE id = $1",
        event_id,
        code
    )
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(player)
}
