use crate::domain::{NewPlayer, PlayerEmail, PlayerName};
use crate::events::lock_active_event;
use crate::routes::errors::database_error;
use actix_web::post;
use actix_web::{HttpResponse, web};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct FormData {
    /// Nonblank name, at most 256 graphemes; cannot contain / ( ) \" < > \\ { }.
    #[schema(example = "Alex")]
    name: String,
    /// Optional valid email address.
    #[schema(example = "alex@example.com")]
    email: Option<String>,
}

impl TryFrom<FormData> for NewPlayer {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        let name = PlayerName::parse(value.name)?;
        let email = value.email.map(PlayerEmail::parse).transpose()?;
        Ok(NewPlayer { email, name })
    }
}

#[utoipa::path(
    tag = "Players", summary = "Register a player in the active event",
    description = "Creates a player and allocates a four-digit PDN. Submit URL-encoded form data, not JSON.",
    request_body(content = FormData, content_type = "application/x-www-form-urlencoded", example = json!({"name": "Alex", "email": "alex@example.com"})),
    responses((status = 200, description = "Player registered; empty body"),
        (status = 400, description = "Missing or invalid name or email"),
        (status = 409, description = "No active event, duplicate name in the active event, or no PDN codes remaining. Text body without a Content-Type header.", body = String, content_type = "text/plain"),
        (status = 500, description = "Database operation failed", body = String, content_type = "text/plain"))
)]
#[post("/players")]
#[tracing::instrument(
    name = "Registering player",
    skip(form, pool),
    fields(
        name = %form.name,
        email = %form.email.is_some(),
    )
)]
pub async fn register_player(form: web::Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    let new_player = match form.0.try_into() {
        Ok(player) => player,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    match insert_player(&pool, &new_player).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(RegistrationError::NoActiveEvent) => HttpResponse::Conflict().body("No active event"),
        Err(RegistrationError::CodesExhausted) => {
            HttpResponse::Conflict().body("No PDN codes available for the active event")
        }
        Err(RegistrationError::Database(error)) => {
            if error.as_database_error().and_then(|e| e.constraint())
                == Some("players_event_name_key")
            {
                return HttpResponse::Conflict()
                    .body("Player name already exists in the active event");
            }
            database_error(error, "Failed to register player").error_response()
        }
    }
}

#[derive(Debug)]
pub enum RegistrationError {
    NoActiveEvent,
    CodesExhausted,
    Database(sqlx::Error),
}

impl From<sqlx::Error> for RegistrationError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

#[tracing::instrument(name = "Adding a player to the database", skip(new_player, pool))]
pub async fn insert_player(pool: &PgPool, new_player: &NewPlayer) -> Result<(), RegistrationError> {
    let mut transaction = pool.begin().await?;
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

    sqlx::query!(
        "INSERT INTO players (id, event_id, pdn_code, name, email, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
        Uuid::new_v4(), event_id, code, new_player.name.as_ref(),
        new_player.email.as_ref().map(|email| email.as_ref()), Utc::now()
    )
    .execute(&mut *transaction)
    .await?;
    sqlx::query!(
        "UPDATE events SET next_pdn_code = $2::text::integer + 1 WHERE id = $1",
        event_id,
        code
    )
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}
